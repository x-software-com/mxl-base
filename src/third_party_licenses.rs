pub extern crate mithra_lib;

use anyhow::{Context, Result};
use log::*;
pub use mithra_lib::third_party_licenses::{ThirdPartyLibrary, ThirdPartyLicenses};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const MXL_THIRD_PARTY_LICENSES_PATH_ENV_NAME: &str = "MXL_THIRD_PARTY_LICENSES_PATH";

fn default_search_path() -> PathBuf {
    match env::var(MXL_THIRD_PARTY_LICENSES_PATH_ENV_NAME) {
        Ok(var) => Path::new(&var).to_path_buf(),
        Err(_) => env::current_exe()
            .expect("Failed to get current executable")
            .parent()
            .expect("Failed to get parent directory of executable")
            .join(Path::new("../share/licenses")),
    }
}

pub struct Builder {
    search_path: PathBuf,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            search_path: default_search_path(),
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn set_search_path(mut self, path: &Path) -> Self {
        self.search_path = path.to_path_buf();
        self
    }

    pub fn build(self) -> Result<Licenses> {
        let json_files = Self::find_json_files(&self.search_path)?;
        let mut third_party_libs = vec![];

        for file in json_files {
            let third_party_licenses = ThirdPartyLicenses::load(file.as_path())?;

            for tpl in &third_party_licenses.third_party_libraries {
                if !third_party_libs.contains(tpl) {
                    third_party_libs.push(tpl.clone());
                }
            }
        }

        Ok(Licenses { third_party_libs })
    }

    fn find_json_files(path: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut files = vec![];

        for entry in fs::read_dir(path).with_context(|| {
            format!(
                "Cannot read third party licenses from directory '{}'",
                path.to_string_lossy()
            )
            .clone()
        })? {
            let entry = entry.with_context(|| {
                format!(
                    "Cannot read third party licenses entry from directory '{}'",
                    path.to_string_lossy()
                )
                .clone()
            })?;
            let path = entry.path().clone();

            if let Some(ext) = path.extension() {
                if ext == "json" {
                    debug!("Add license file {path:?}");
                    files.push(path)
                } else {
                    trace!("Ignore directory entry {path:?}");
                }
            }
        }

        Ok(files)
    }
}

#[derive(Default)]
pub struct Licenses {
    third_party_libs: Vec<ThirdPartyLibrary>,
}

impl Licenses {
    pub fn libs(&self) -> &Vec<ThirdPartyLibrary> {
        &self.third_party_libs
    }

    pub fn print(&self) -> Result<()> {
        use std::collections::HashMap;

        struct PackageInfo {
            name: String,
            version: String,
            license: String,
        }

        impl std::fmt::Display for PackageInfo {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {} ({})", self.name, self.version, self.license)
            }
        }

        struct LicenseInfo {
            license: String,
            packages: Vec<PackageInfo>,
        }

        let mut license_map: HashMap<(String, String), LicenseInfo> = HashMap::new();

        for lib in &self.third_party_libs {
            for license in &lib.licenses {
                let package_infos = PackageInfo {
                    name: lib.package_name.clone(),
                    version: lib.package_version.clone(),
                    license: lib.license.clone(),
                };

                if let Some(license_info) = license_map.get_mut(&(license.license.clone(), license.text.clone())) {
                    license_info.packages.push(package_infos);
                } else {
                    license_map.insert(
                        (license.license.clone(), license.text.clone()),
                        LicenseInfo {
                            license: license.license.clone(),
                            packages: vec![package_infos],
                        },
                    );
                }
            }
        }

        println!();
        for ((_license_id, text), license_info) in license_map {
            println!("────────────────────────────────────────────────────────────────────────────────────────");
            println!("{}", license_info.license,);
            println!("────────────────────────────────────────────────────────────────────────────────────────\n");
            println!(
                "Used by: {}\n",
                license_info
                    .packages
                    .iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<String>>()
                    .join(", ")
            );

            println!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
        }
        Ok(())
    }
}
