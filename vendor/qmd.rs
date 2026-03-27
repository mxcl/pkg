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
    // qmd needs brew’s sqlite: indeed. macOS sqlite has no extension support.
    &["node", "sqlite"]
}

pub fn executables() -> &'static [&'static str] {
    &["qmd"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("tobi/qmd")?;
    parse_semver(tag.strip_prefix('v').unwrap_or(&tag), "qmd")
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::NpmGlobal {
        package: "@tobilu/qmd".to_string(),
    }
}
