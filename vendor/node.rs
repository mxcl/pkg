use semver::Version;

use crate::vendor::{InstallStrategy, VendorEntry, github_latest_tag, parse_semver};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "node",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["node", "npm", "npx"] // corepack is not included for some reason
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("nodejs/node")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix('v').unwrap_or(version), "node")
}

pub fn download_url(version: &Version) -> String {
    let version = format!("v{version}");
    format!("https://nodejs.org/dist/{version}/node-{version}-darwin-arm64.tar.gz")
}

pub fn install(version: &Version) -> InstallStrategy {
    InstallStrategy::CopyTree {
        source: format!("node-v{version}-darwin-arm64"),
    }
}
