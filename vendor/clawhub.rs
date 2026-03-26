use semver::Version;

use crate::vendor::{InstallStrategy, VendorEntry, npm_latest_tag, parse_semver};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "clawhub",
    dependencies: Some(dependencies),
    executables,
    version,
    download_url: None,
    install,
};

pub fn dependencies() -> &'static [&'static str] {
    &["node"]
}

pub fn executables() -> &'static [&'static str] {
    &["clawhub"]
}

pub fn version() -> Result<Version, String> {
    let version = npm_latest_tag("clawhub")?;
    parse_semver(&version, "clawhub")
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::NpmGlobal {
        package: "clawhub".to_string(),
    }
}
