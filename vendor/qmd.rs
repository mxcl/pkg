use semver::Version;

use crate::vendor::{InstallStrategy, VendorEntry, github_latest_tag, parse_semver};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "qmd",
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
    &["qmd"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("openclaw/openclaw")?;
    parse_semver(&tag, "qmd")
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::NpmGlobal {
        package: "openclaw".to_string(),
    }
}
