use semver::Version;

use crate::vendor::{
    InstallStrategy, VendorEntry, github_latest_tag, github_release_url, parse_semver,
};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "yoink",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["yoink"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("mxcl/yoink")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix('v').unwrap_or(version), "yoink")
}

pub fn download_url(version: &Version) -> String {
    github_release_url(
        "mxcl/yoink",
        &format!("v{version}"),
        &format!("yoink-{version}-Darwin-arm64.tar.gz"),
    )
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: "yoink".to_string(),
        destination_dir: "bin".to_string(),
        destination_name: None,
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
