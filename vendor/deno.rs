use semver::Version;

use crate::vendor::{
    InstallStrategy, VendorEntry, github_latest_tag, github_release_url, parse_semver,
};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "deno",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["deno"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("denoland/deno")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix('v').unwrap_or(version), "deno")
}

pub fn download_url(version: &Version) -> String {
    github_release_url(
        "denoland/deno",
        &format!("v{version}"),
        "deno-aarch64-apple-darwin.zip",
    )
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: "deno".to_string(),
        destination_dir: "bin".to_string(),
        destination_name: None,
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
