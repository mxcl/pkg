use std::io::Read;

use semver::Version;
use serde::Deserialize;
use ureq::Error as UreqError;

pub struct VendorPackage {
    pub name: &'static str,
    pub dependencies: &'static [&'static str],
    pub executables: &'static [&'static str],
    pub version: fn() -> Result<Version, String>,
    pub download_url: Option<fn(&Version) -> String>,
    pub install: fn(&Version) -> InstallStrategy,
}

#[derive(Clone)]
pub enum InstallStrategy {
    NpmGlobal {
        package: String,
    },
    CopyFile {
        source: String,
        destination_dir: String,
        destination_name: Option<String>,
        mode: u32,
        create_dirs: Vec<String>,
    },
    CopyTree {
        source: String,
    },
}

pub struct VendorEntry {
    pub name: &'static str,
    pub dependencies: Option<fn() -> &'static [&'static str]>,
    pub executables: fn() -> &'static [&'static str],
    pub version: fn() -> Result<Version, String>,
    pub download_url: Option<fn(&Version) -> String>,
    pub install: fn(&Version) -> InstallStrategy,
}

impl VendorEntry {
    pub fn package(&self) -> VendorPackage {
        VendorPackage {
            name: self.name,
            dependencies: self.dependencies.map(|f| f()).unwrap_or(&[]),
            executables: (self.executables)(),
            version: self.version,
            download_url: self.download_url,
            install: self.install,
        }
    }
}

pub fn github_release_url(repo: &str, tag: &str, asset: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
}

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn github_api_root() -> String {
    std::env::var("PKG_GITHUB_API_ROOT")
        .unwrap_or_else(|_| "https://api.github.com".to_string())
}

fn npm_registry_root() -> String {
    std::env::var("PKG_NPM_REGISTRY_ROOT")
        .unwrap_or_else(|_| "https://registry.npmjs.org".to_string())
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

#[derive(Deserialize)]
struct NpmPackageVersion {
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
}

#[derive(Deserialize)]
struct NpmDistTags {
    latest: String,
}

pub fn github_latest_tag(repo: &str) -> Result<String, String> {
    let url = format!("{}/repos/{repo}/releases/latest", github_api_root());
    let release: GithubRelease =
        fetch_json(&url, &format!("failed to fetch latest release for {repo}"))?;
    Ok(release.tag_name)
}

pub fn npm_latest_tag(package: &str) -> Result<String, String> {
    let url = format!(
        "{}/{}",
        npm_registry_root(),
        urlencoding::encode(package)
    );
    let package: NpmPackageVersion =
        fetch_json(&url, &format!("failed to fetch npm metadata for {package}"))?;
    Ok(package.dist_tags.latest)
}

pub fn parse_semver(version: &str, context: &str) -> Result<Version, String> {
    Version::parse(version)
        .map_err(|err| format!("failed to parse semver {version} for {context}: {err}"))
}

fn fetch_json<T>(url: &str, context: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| match err {
            UreqError::Status(code, _) => format!("{context}: http {code}"),
            UreqError::Transport(err) => format!("{context}: {err}"),
        })?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("{context}: {err}"))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("{context}: {err}"))
}

#[path = "../vendor/bun.rs"]
pub mod bun;
#[path = "../vendor/clawhub.rs"]
pub mod clawhub;
#[path = "../vendor/codex.rs"]
pub mod codex;
#[path = "../vendor/deno.rs"]
pub mod deno;
#[path = "../vendor/gh.rs"]
pub mod gh;
#[path = "../vendor/node.rs"]
pub mod node;
#[path = "../vendor/openclaw.rs"]
pub mod openclaw;
#[path = "../vendor/qmd.rs"]
pub mod qmd;

pub static PACKAGES: &[&VendorEntry] = &[
    &bun::ENTRY,
    &clawhub::ENTRY,
    &codex::ENTRY,
    &deno::ENTRY,
    &gh::ENTRY,
    &node::ENTRY,
    &openclaw::ENTRY,
    &qmd::ENTRY,
];

pub fn get(name: &str) -> Option<VendorPackage> {
    PACKAGES
        .iter()
        .copied()
        .find(|entry| entry.name == name)
        .map(VendorEntry::package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_registry_contains_all_packages() {
        let mut names = PACKAGES.iter().map(|entry| entry.name).collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "bun", "clawhub", "codex", "deno", "gh", "node", "openclaw", "qmd"
            ]
        );
    }

    #[test]
    fn vendor_packages_expose_executables() {
        assert_eq!(get("bun").unwrap().executables, ["bun"]);
        assert_eq!(get("clawhub").unwrap().executables, ["clawhub"]);
        assert_eq!(get("codex").unwrap().executables, ["codex"]);
        assert_eq!(get("deno").unwrap().executables, ["deno"]);
        assert_eq!(get("gh").unwrap().executables, ["gh"]);
        assert_eq!(get("node").unwrap().executables, ["node", "npm", "npx"]);
        assert_eq!(get("openclaw").unwrap().executables, ["openclaw"]);
        assert_eq!(get("qmd").unwrap().executables, ["qmd"]);
    }

    #[test]
    fn vendor_packages_return_semver() {
        assert_eq!(
            bun::parse_version("bun-v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
        assert_eq!(
            codex::parse_version("rust-v0.116.0").unwrap(),
            Version::parse("0.116.0").unwrap()
        );
        assert_eq!(
            gh::parse_version("v2.88.1").unwrap(),
            Version::parse("2.88.1").unwrap()
        );
        assert_eq!(
            deno::parse_version("v2.7.7").unwrap(),
            Version::parse("2.7.7").unwrap()
        );
        assert_eq!(
            node::parse_version("v22.18.0").unwrap(),
            Version::parse("22.18.0").unwrap()
        );
    }

    #[test]
    fn vendor_packages_compute_download_urls_in_code() {
        let bun = get("bun").unwrap();
        let codex = get("codex").unwrap();
        let deno = get("deno").unwrap();
        let gh = get("gh").unwrap();
        let node = get("node").unwrap();
        let bun_version = Version::parse("1.2.3").unwrap();
        let codex_version = Version::parse("0.116.0").unwrap();
        let deno_version = Version::parse("2.7.7").unwrap();
        let gh_version = Version::parse("2.88.1").unwrap();
        let node_version = Version::parse("22.18.0").unwrap();

        assert_eq!(
            bun.download_url.unwrap()(&bun_version),
            "https://github.com/oven-sh/bun/releases/download/bun-v1.2.3/bun-darwin-aarch64.zip"
        );
        assert_eq!(
            codex.download_url.unwrap()(&codex_version),
            "https://github.com/openai/codex/releases/download/rust-v0.116.0/codex-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(
            deno.download_url.unwrap()(&deno_version),
            "https://github.com/denoland/deno/releases/download/v2.7.7/deno-aarch64-apple-darwin.zip"
        );
        assert_eq!(
            gh.download_url.unwrap()(&gh_version),
            "https://github.com/cli/cli/releases/download/v2.88.1/gh_2.88.1_macOS_arm64.zip"
        );
        assert_eq!(
            node.download_url.unwrap()(&node_version),
            "https://nodejs.org/dist/v22.18.0/node-v22.18.0-darwin-arm64.tar.gz"
        );
    }

    #[test]
    fn vendor_packages_expose_dependencies() {
        assert_eq!(get("codex").unwrap().dependencies, ["ripgrep"]);
        assert_eq!(get("qmd").unwrap().dependencies, ["node", "sqlite"]);
    }

    #[test]
    fn codex_installs_platform_binary_as_codex() {
        let version = Version::parse("0.116.0").unwrap();
        let strategy = codex::install(&version);
        match strategy {
            InstallStrategy::CopyFile {
                source,
                destination_dir,
                destination_name,
                mode,
                create_dirs,
            } => {
                assert_eq!(source, "codex-aarch64-apple-darwin");
                assert_eq!(destination_dir, "bin");
                assert_eq!(destination_name.as_deref(), Some("codex"));
                assert_eq!(mode, 0o755);
                assert_eq!(create_dirs, vec!["bin".to_string()]);
            }
            _ => panic!("codex should install from a downloaded binary asset"),
        }
    }

    #[test]
    fn bun_installs_platform_binary_from_archive_subdirectory() {
        let version = Version::parse("1.2.3").unwrap();
        let strategy = bun::install(&version);
        match strategy {
            InstallStrategy::CopyFile {
                source,
                destination_dir,
                destination_name,
                mode,
                create_dirs,
            } => {
                assert_eq!(source, "bun-darwin-aarch64/bun");
                assert_eq!(destination_dir, "bin");
                assert_eq!(destination_name, None);
                assert_eq!(mode, 0o755);
                assert_eq!(create_dirs, vec!["bin".to_string()]);
            }
            _ => panic!("bun should install from the extracted archive directory"),
        }
    }
}
