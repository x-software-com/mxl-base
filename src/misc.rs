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

pub fn project_dirs() -> &'static directories::ProjectDirs {
    static PROJECT_DIR: OnceCell<directories::ProjectDirs> = OnceCell::new();
    PROJECT_DIR.get_or_init(|| {
        let about = super::about::about();
        if let Some(dir) = directories::ProjectDirs::from(about.qualifier, about.organization, about.app_name) {
            dir
        } else {
            panic!("Cannot determine project directories")
        }
    })
}
