use once_cell::sync::OnceCell;

pub fn init(
    qualifier: &'static str,
    organization: &'static str,
    app_name: &'static str,
    binary_name: &'static str,
    version: &'static str,
) {
    crate::about::about_init(qualifier, organization, app_name, binary_name, version);
    crate::localization::init();
}

static PROJECT_DIR: OnceCell<directories::ProjectDirs> = OnceCell::new();

#[cfg(feature = "with_test")]
pub fn init_test() {
    use once_cell::sync::Lazy;
    use tempfile::TempDir;

    static TMP_DIR: Lazy<TempDir> = Lazy::new(|| TempDir::new().expect("Failed create tmp directory"));
    let path = TMP_DIR.path();

    PROJECT_DIR
        .set(directories::ProjectDirs::from_path(path.to_path_buf()).expect("No directories?"))
        .expect("Already initialized");

    crate::proc_dir::set_proc_dir(path.to_path_buf());
}

pub fn project_dirs() -> &'static directories::ProjectDirs {
    PROJECT_DIR.get_or_init(|| {
        let about = super::about::about();
        if let Some(dir) = directories::ProjectDirs::from(about.qualifier, about.organization, about.app_name) {
            dir
        } else {
            panic!("Cannot determine project directories")
        }
    })
}
