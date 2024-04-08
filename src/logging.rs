use crate::localization::helper::fl;
use anyhow::{Context, Result};
use log::*;
use once_cell::sync::{Lazy, OnceCell};
use std::path::{Path, PathBuf};

const KEEP_NUMBER_OF_FILES: usize = 20;
const DEFAULT_LEVEL: log::LevelFilter = log::LevelFilter::Trace;
const LOG_FILE_EXTENSION: &str = "log";
const LOG_DIR_GENERIC: &str = "log";
const LOG_FILE_FMT: &str = const_format::formatcp!("%Y-%m-%d_%H_%M_%S.{}", LOG_FILE_EXTENSION);

#[cfg(debug_assertions)] // Set debug level for console in debug builds
const CONSOLE_LEVEL: log::LevelFilter = log::LevelFilter::Trace;

#[cfg(not(debug_assertions))] // Set debug level for console in release builds
const CONSOLE_LEVEL: log::LevelFilter = log::LevelFilter::Warn;

static LOG_RECEIVER_LOG_LEVEL: Lazy<std::sync::RwLock<log::LevelFilter>> =
    Lazy::new(|| std::sync::RwLock::new(DEFAULT_LEVEL));

pub fn set_log_level(level: log::LevelFilter) {
    *LOG_RECEIVER_LOG_LEVEL.write().unwrap() = level;
}

pub fn get_log_level() -> log::LevelFilter {
    *LOG_RECEIVER_LOG_LEVEL.read().unwrap()
}

static CURRENT_LOG_FILE_HOLDER: OnceCell<PathBuf> = OnceCell::new();
pub fn current_log_file() -> &'static PathBuf {
    CURRENT_LOG_FILE_HOLDER.get().expect("init() must be called first")
}

pub struct Builder {
    logger: Option<fern::Dispatch>,
    without_stderr: bool,
    without_generic_log_dir: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            logger: Some(fern::Dispatch::new().level(DEFAULT_LEVEL)),
            without_stderr: false,
            without_generic_log_dir: false,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level_for<T: Into<std::borrow::Cow<'static, str>>>(mut self, module: T, level: log::LevelFilter) -> Self {
        self.logger = Some(self.logger.unwrap().level_for(module, level));
        self
    }

    pub fn without_stderr(mut self) -> Self {
        self.without_stderr = true;
        self
    }

    pub fn without_generic_log_dir(mut self) -> Self {
        self.without_generic_log_dir = true;
        self
    }

    fn generic_log_dir(&self) -> &'static PathBuf {
        static DIR: OnceCell<PathBuf> = OnceCell::new();
        DIR.get_or_init(|| {
            super::misc::project_dirs()
                .data_local_dir()
                .join(std::path::Path::new(LOG_DIR_GENERIC))
        })
    }

    fn generig_log_file(&self) -> &'static PathBuf {
        static NAME: OnceCell<PathBuf> = OnceCell::new();
        NAME.get_or_init(|| {
            self.generic_log_dir().join(format!(
                "{}_{}",
                super::about::about().binary_name,
                chrono::Local::now().format(LOG_FILE_FMT)
            ))
        })
    }

    fn build_with_panic_on_failure(&mut self, log_dir: &Path) {
        // NOTE!!!
        // Every error MUST be a panic here else the user will not be able to see the error!
        let mut logger = self
            .logger
            .take()
            .unwrap()
            .filter(|metadata| metadata.level() <= *LOG_RECEIVER_LOG_LEVEL.read().unwrap())
            .format(|out, message, record| {
                out.finish(format_args!("{} [{}] {}", record.level(), record.target(), message))
            });
        let log_file = CURRENT_LOG_FILE_HOLDER
            .get_or_init(|| log_dir.join(format!("{}.{}", super::about::about().binary_name, LOG_FILE_EXTENSION)));

        std::fs::create_dir_all(log_dir).unwrap_or_else(|error| {
            panic!(
                "Cannot create logging directory '{}': {:?}",
                log_dir.to_string_lossy(),
                error
            )
        });
        logger = logger.chain(
            fern::log_file(log_file)
                .unwrap_or_else(|error| panic!("Cannot open log file '{}': {:?}", log_file.to_string_lossy(), error)),
        );
        if !self.without_stderr {
            logger = logger.chain(fern::Dispatch::new().level(CONSOLE_LEVEL).chain(std::io::stderr()));
        }
        if !self.without_generic_log_dir {
            let log_dir = self.generic_log_dir();
            std::fs::create_dir_all(log_dir).unwrap_or_else(|error| {
                panic!(
                    "Cannot create logging directory '{}': {:?}",
                    log_dir.to_string_lossy(),
                    error
                )
            });
            let log_file = self.generig_log_file();
            logger =
                logger.chain(fern::log_file(log_file).unwrap_or_else(|error| {
                    panic!("Cannot open log file '{}': {:?}", log_file.to_string_lossy(), error)
                }));
        }
        logger.apply().expect("Cannot start logging");
    }

    fn cleanup_logfiles(binary_name: &str, path: &std::path::Path) -> Result<()> {
        // Collect all matching logfiles in the directory:
        let log_file_extension = std::ffi::OsString::from(LOG_FILE_EXTENSION);
        let mut log_files = std::fs::read_dir(path)
            .with_context(|| format!("Cannot list log directory '{}'", path.to_string_lossy()))?
            .filter_map(|file| {
                match file {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file()
                            && !path.is_symlink()
                            && path.starts_with(binary_name)
                            && path.extension() == Some(log_file_extension.as_os_str())
                        {
                            return Some(path);
                        }
                    }
                    Err(error) => warn!("Cannot read log file: {}", error.to_string()),
                }
                None
            })
            .collect::<Vec<_>>();

        // Remove all logfiles that exceed the number of files to preserve:
        if log_files.len() > KEEP_NUMBER_OF_FILES {
            log_files.sort();
            let mut len = log_files.len();
            for file in log_files.iter() {
                match std::fs::remove_file(file) {
                    Ok(_) => {
                        trace!("Removed logfile {file:?}");
                        len -= 1;
                        if len <= KEEP_NUMBER_OF_FILES {
                            break;
                        }
                    }
                    Err(error) => warn!("Cannot remove log file '{}': {}", file.to_string_lossy(), error),
                }
            }
        }
        Ok(())
    }

    pub fn build(mut self, log_dir: &Path) -> Result<()> {
        self.build_with_panic_on_failure(log_dir);
        let about = super::about::about();

        if !self.without_generic_log_dir {
            #[cfg(target_family = "unix")]
            {
                let log_dir = self.generic_log_dir();
                let symlink = log_dir.join(format!("{}.{}", about.binary_name, LOG_FILE_EXTENSION));
                _ = std::fs::remove_file(&symlink);
                let log_file = self.generig_log_file();
                _ = std::os::unix::fs::symlink(log_file, &symlink);
            }
        }
        let log_file = current_log_file();
        if !self.without_stderr {
            // Currently not use translation. The log file name is wrapped to: <2068>log-file-name<2069>
            // Some terminals copy these possibly invisible special characters when selecting and copying,
            // so that the log file cannot be opened.
            if false {
                println!("{}", fl!("log-written-to", file_name = log_file.to_string_lossy()));
            } else {
                println!("Log is written to '{}'", log_file.to_string_lossy());
            }
        }

        info!("{} {}", about.app_name, about.version);
        info!("Log is written to '{}'", log_file.to_string_lossy());

        if !self.without_generic_log_dir {
            Self::cleanup_logfiles(about.binary_name, self.generic_log_dir().as_path())?;
        }

        Ok(())
    }
}
