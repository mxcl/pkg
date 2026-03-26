use semver::Version;

use crate::vendor::{
    InstallStrategy, VendorEntry, github_latest_tag, github_release_url, parse_semver,
};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "gh",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["gh"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("cli/cli")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix('v').unwrap_or(version), "gh")
}

pub fn download_url(version: &Version) -> String {
    github_release_url(
        "cli/cli",
        &format!("v{version}"),
        &format!("gh_{version}_macOS_arm64.zip"),
    )
}

pub fn install(version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: format!("gh_{version}_macOS_arm64/bin/gh"),
        destination_dir: "bin".to_string(),
        destination_name: None,
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
