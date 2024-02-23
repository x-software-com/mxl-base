use crate::localization::helper::fl;
use anyhow::{Context, Result};
use fs4::FileExt;
use once_cell::sync::OnceCell;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

pub const ARCHIVE_DEFAULT_FILE_SUFFIX: &str = "zip";
pub const ARCHIVE_MIME_TYPE: &str = "application/x-zip";

const CURRENT_DIR_FMT: &str = "%Y-%m-%d_%H_%M_%S";
const PROC_DIR_NAME: &str = "proc";
const PROC_FAILED_DIR_NAME: &str = "proc_failed";
const LOCK_FILE_NAME: &str = "run.lock";

static RUN_DIR_HOLDER: OnceCell<PathBuf> = OnceCell::new();

fn create_dir_all_with_panic<P: AsRef<Path> + std::fmt::Debug>(path: P) {
    std::fs::create_dir_all(&path).unwrap_or_else(|error| panic!("Cannot create directory {:?}: {:?}", path, error));
}

pub fn set_proc_dir(path: PathBuf) {
    RUN_DIR_HOLDER.set(path).expect("Proc directory already set");
    create_dir_all_with_panic(RUN_DIR_HOLDER.get().unwrap());
}

pub fn default_proc_dir() -> &'static PathBuf {
    static HOLDER: OnceCell<PathBuf> = OnceCell::new();
    HOLDER.get_or_init(|| {
        super::misc::project_dirs()
            .data_local_dir()
            .join(std::path::Path::new(PROC_DIR_NAME))
    })
}

pub fn default_failed_dir() -> &'static PathBuf {
    static HOLDER: OnceCell<PathBuf> = OnceCell::new();
    HOLDER.get_or_init(|| {
        let failed_runs_path = super::misc::project_dirs().data_local_dir().join(PROC_FAILED_DIR_NAME);
        create_dir_all_with_panic(&failed_runs_path);
        failed_runs_path
    })
}

fn move_to_failed_dir() -> Result<()> {
    // Move failed runs out of the run directory:
    let failed_dir = default_failed_dir();
    let proc_dir = default_proc_dir();
    if let Ok(entry) = std::fs::read_dir(proc_dir) {
        for entry in entry {
            let entry =
                entry.with_context(|| format!("Cannot list directories in '{}'", proc_dir.to_string_lossy()))?;

            let preserve_dir = |from: &Path| {
                let from_dir_name = from
                    .file_name()
                    .unwrap_or_else(|| panic!("Cannot get name of path '{:?}'", from));
                let to = failed_dir.join(from_dir_name);
                std::fs::rename(from, &to).with_context(|| {
                    format!(
                        "Cannot move failed run data directory from '{}' to '{}'",
                        from.to_string_lossy(),
                        to.to_string_lossy()
                    )
                })
            };

            let existing_run_dir = entry.path();
            let lock_file_path = existing_run_dir.join(LOCK_FILE_NAME);

            match File::open(&lock_file_path)
                .with_context(|| format!("Cannot open lock file '{}'", lock_file_path.to_string_lossy()))
            {
                Ok(lock_file) => match lock_file.try_lock_exclusive() {
                    Ok(()) => {
                        preserve_dir(&existing_run_dir)?;
                    }
                    Err(_error) => {
                        // Cannot get lock - directory is in use
                    }
                },
                Err(error) => match error.downcast_ref::<std::io::Error>() {
                    Some(err) => {
                        if std::io::ErrorKind::NotFound == err.kind() {
                            preserve_dir(&existing_run_dir)?;
                        } else {
                            return Err(error);
                        }
                    }
                    _ => return Err(error),
                },
            }
        }
    }
    Ok(())
}

struct LockFile(File, PathBuf);

impl Drop for LockFile {
    fn drop(&mut self) {
        // LockFile is not removed if the process is killed via SIGKILL
        let file_name = &self.1;
        _ = std::fs::remove_file(file_name);
    }
}

fn create_lock_file(path: &Path) -> &'static Result<LockFile> {
    static HOLDER: OnceCell<Result<LockFile>> = OnceCell::new();
    HOLDER.get_or_init(|| {
        let lock_file_path = path.join(LOCK_FILE_NAME);

        let lock_file = File::create(&lock_file_path)
            .with_context(|| format!("Cannot create file '{}'", lock_file_path.to_string_lossy()))?;
        lock_file
            .try_lock_exclusive()
            .with_context(|| format!("Cannot lock exclusive file '{}'", lock_file_path.to_string_lossy()))?;
        Ok(LockFile(lock_file, lock_file_path))
    })
}

pub fn proc_dir() -> &'static PathBuf {
    RUN_DIR_HOLDER.get_or_init(|| {
        let data_dir = chrono::Local::now().format(CURRENT_DIR_FMT).to_string();
        let data_dir = default_proc_dir().join(std::path::Path::new(&data_dir));

        create_dir_all_with_panic(&data_dir);
        if let Err(err) = create_lock_file(&data_dir) {
            panic!("Cannot lock directory: {:?}", err);
        }
        move_to_failed_dir().unwrap_or_else(|error| panic!("Cannot move failed runs: {:?}", error));
        data_dir
    })
}

pub fn remove_proc_dir() -> std::io::Result<()> {
    if let Some(data_dir) = RUN_DIR_HOLDER.get() {
        std::fs::remove_dir_all(data_dir)?;
    }
    Ok(())
}

fn create_archive(src_dirs: &[PathBuf], archive_file_path: &Path) -> Result<()> {
    if src_dirs.is_empty() {
        anyhow::bail!("Cannot archive empty list of directories");
    }

    let archive_file = File::create(archive_file_path)
        .with_context(|| format!("Cannot create archive '{}'", archive_file_path.to_string_lossy()))?;

    let mut zip = ZipWriter::new(archive_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Bzip2);

    for src_dir in src_dirs {
        let parent_dir = src_dir.parent().unwrap_or_else(|| src_dir);
        let walk_dir = WalkDir::new(src_dir);
        let it = walk_dir.into_iter().filter_map(|e| e.ok());
        let mut buffer = Vec::new();

        for entry in it {
            let path = entry.path();
            let name = path.strip_prefix(parent_dir).unwrap();

            // Write file or directory explicitly
            // Some unzip tools unzip files with directory paths correctly, some do not!
            if path.is_file() {
                log::trace!("adding file {path:?} as {name:?} ...");
                #[allow(deprecated)]
                zip.start_file_from_path(name, options)
                    .with_context(|| format!("Cannot add file '{}' to archive", name.to_string_lossy()))?;
                let mut f = File::open(path).with_context(|| {
                    format!(
                        "Cannot open file '{}' to add it to the archive.",
                        path.to_string_lossy()
                    )
                })?;

                f.read_to_end(&mut buffer).with_context(|| {
                    format!(
                        "Cannot read from file '{}' to add it to the archive.",
                        path.to_string_lossy()
                    )
                })?;
                zip.write_all(&buffer).with_context(|| {
                    format!("Cannot write file buffer '{}' to the archive.", path.to_string_lossy())
                })?;
                buffer.clear();
            } else if path.is_dir() && !name.as_os_str().is_empty() {
                // Only if not root! Avoids path spec / warning
                // and mapname conversion failed error on unzip
                log::trace!("adding dir {path:?} as {name:?} ...");
                zip.add_directory(name.to_string_lossy(), options)
                    .with_context(|| format!("Cannot add directory '{}' to the archive", name.to_string_lossy()))?;
            }
        }
    }

    zip.finish()
        .with_context(|| format!("Cannot finish archive '{}'", archive_file_path.to_string_lossy()))?;

    Ok(())
}

pub fn failed_procs_is_empty() -> Result<bool> {
    Ok(default_failed_dir().read_dir()?.next().is_none())
}

pub fn create_problem_report_file_name() -> String {
    format!(
        "{}_problem_report.{}",
        super::about::about().binary_name,
        ARCHIVE_DEFAULT_FILE_SUFFIX
    )
}

pub fn failed_procs_archive_and_remove(archive_file_path: &Path) -> Result<()> {
    let failed_dir = default_failed_dir();
    let directories = std::fs::read_dir(failed_dir)?
        .map(|entry| Ok(entry?.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    if directories.is_empty() {
        println!("{}", fl!("no-bug-reports"));
        return Ok(());
    }
    create_archive(&directories, archive_file_path)?;
    for item in directories {
        if item.is_file() {
            std::fs::remove_file(&item).with_context(|| format!("Cannot remove file '{}'", item.to_string_lossy()))?;
        } else {
            std::fs::remove_dir_all(&item)
                .with_context(|| format!("Cannot remove directory '{}'", item.to_string_lossy()))?;
        }
    }
    println!(
        "{}",
        fl!("bug-report-written-to", file_name = archive_file_path.to_string_lossy())
    );
    Ok(())
}

pub fn failed_procs_move_to_trash() -> Result<()> {
    let failed_dir = default_failed_dir();
    let directories = std::fs::read_dir(failed_dir)?
        .map(|entry| Ok(entry?.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    trash::delete_all(directories).with_context(|| "Cannot move failed executions to trash")
}

pub fn create_report_file_name() -> String {
    format!(
        "{}_report.{}",
        super::about::about().binary_name,
        ARCHIVE_DEFAULT_FILE_SUFFIX
    )
}

pub fn proc_dir_archive(archive_file_path: &Path) -> Result<()> {
    create_archive(&[proc_dir().to_owned()], archive_file_path)
}
