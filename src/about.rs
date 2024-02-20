use once_cell::sync::OnceCell;

#[derive(Debug)]
pub struct About {
    pub qualifier: &'static str,
    pub organization: &'static str,
    pub app_name: &'static str,
    pub binary_name: &'static str,
    pub version: &'static str,
}

static ABOUT_REGISTER: OnceCell<About> = OnceCell::new();

pub(crate) fn about_init(
    qualifier: &'static str,
    organization: &'static str,
    app_name: &'static str,
    binary_name: &'static str,
    version: &'static str,
) {
    ABOUT_REGISTER
        .set(About {
            qualifier,
            organization,
            app_name,
            binary_name,
            version,
        })
        .expect("Already initialized");
}

pub fn about() -> &'static About {
    ABOUT_REGISTER.get().expect("Initialize first")
}
