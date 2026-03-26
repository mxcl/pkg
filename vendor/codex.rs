use semver::Version;

use crate::vendor::{
    InstallStrategy, VendorEntry, github_latest_tag, github_release_url, parse_semver,
};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "codex",
    dependencies: Some(dependencies),
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn dependencies() -> &'static [&'static str] {
    &["ripgrep"]
}

pub fn executables() -> &'static [&'static str] {
    &["codex"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("openai/codex")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix("rust-v").unwrap_or(version), "codex")
}

pub fn download_url(version: &Version) -> String {
    github_release_url(
        "openai/codex",
        &format!("rust-v{version}"),
        "codex-aarch64-apple-darwin.tar.gz",
    )
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: "codex-aarch64-apple-darwin".to_string(),
        destination_dir: "bin".to_string(),
        destination_name: Some("codex".to_string()),
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
