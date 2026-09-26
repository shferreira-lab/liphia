// liphia_manifest/src/lib.rs
//
// Data model of the Liphia toolchain files:
//
//   liphia.toml     a project: its name and the packages it depends on
//   liphia.lock     the exact versions installed, including transitive ones
//   package.toml    a package: version, engine compatibility, files, native lib
//   index.toml      every published version of every official package
//
// Version requirements follow npm: "1.2.9" is exact, "^1.2.9" accepts
// compatible versions (same major), "~1.2.9" accepts patches (same minor),
// "*" or "latest" accepts anything. The semver crate treats a bare "1.2.9"
// as caret, so `parse_req` adds the "=" that the npm meaning requires.
//
// This crate does no I/O beyond reading and writing these files; networking
// and installation live in liphia_cli.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use semver::{Version, VersionReq};

// ── Version requirements ──────────────────────────────────────────────────────

pub fn parse_req(text: &str) -> Result<VersionReq, String> {
    let text = text.trim();
    let normalized = if text.is_empty() || text == "latest" {
        "*".to_string()
    } else if text.starts_with(|c: char| c.is_ascii_digit()) {
        format!("={}", text)
    } else {
        text.to_string()
    };
    VersionReq::parse(&normalized).map_err(|e| format!("invalid version requirement '{}': {}", text, e))
}

pub fn parse_version(text: &str) -> Result<Version, String> {
    Version::parse(text.trim()).map_err(|e| format!("invalid version '{}': {}", text, e))
}

// Highest version in `available` that satisfies `req`. Unparseable entries
// are skipped rather than failing the whole resolution.
pub fn highest_matching(available: &[String], req: &VersionReq) -> Option<Version> {
    available
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .max()
}

// ── liphia.toml ───────────────────────────────────────────────────────────────

pub const PROJECT_FILE: &str = "liphia.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub package: ProjectInfo,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    #[serde(default = "default_project_version")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

fn default_project_version() -> String {
    "0.1.0".to_string()
}

// A dependency is either a bare requirement ("^1.0.0") or a table when
// subpackages are selected: { version = "^2.0.0", subpackages = ["sqlite"] }.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Req(String),
    Detailed {
        version: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subpackages: Vec<String>,
    },
}

impl Dependency {
    pub fn req(&self) -> &str {
        match self {
            Dependency::Req(r) => r,
            Dependency::Detailed { version, .. } => version,
        }
    }

    pub fn subpackages(&self) -> &[String] {
        match self {
            Dependency::Req(_) => &[],
            Dependency::Detailed { subpackages, .. } => subpackages,
        }
    }

    // Collapses back to the short form when no subpackage is selected.
    pub fn new(req: String, mut subpackages: Vec<String>) -> Self {
        subpackages.sort();
        subpackages.dedup();
        if subpackages.is_empty() {
            Dependency::Req(req)
        } else {
            Dependency::Detailed { version: req, subpackages }
        }
    }
}

impl ProjectManifest {
    pub fn new(name: &str) -> Self {
        ProjectManifest {
            package: ProjectInfo {
                name: name.to_string(),
                version: default_project_version(),
                entry: None,
            },
            dependencies: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        toml::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))
    }

    // Rewrites the whole file; comments in liphia.toml are not preserved.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| format!("{}: {}", path.display(), e))
    }
}

// ── package.toml ──────────────────────────────────────────────────────────────

pub const PACKAGE_FILE: &str = "package.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeInfo>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub subpackages: BTreeMap<String, Subpackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    // Engine versions this package works with. Native packages pin one
    // exact engine ("=2.0.0") because their library uses the Rust ABI.
    pub engine: String,
    pub entry: String,
    pub files: Vec<String>,
}

// Present only in native packages. `lib` is the library name without
// platform prefix or suffix; `abi` names the calling convention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeInfo {
    pub abi: String,
    pub lib: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subpackage {
    pub files: Vec<String>,
}

impl PackageManifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("package.toml: {}", e))
    }

    pub fn is_native(&self) -> bool {
        self.native.is_some()
    }

    pub fn engine_req(&self) -> Result<VersionReq, String> {
        parse_req(&self.package.engine)
    }
}

// ── liphia.lock ───────────────────────────────────────────────────────────────

pub const LOCK_FILE: &str = "liphia.lock";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(default = "default_lock_version")]
    pub lock_version: u32,
    #[serde(default)]
    pub engine: String,
    #[serde(default, rename = "package")]
    pub packages: Vec<LockedPackage>,
}

fn default_lock_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub native: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subpackages: Vec<String>,
}

impl Lockfile {
    pub fn load(path: &Path) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let body = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let text = format!(
            "# This file is generated by `liphia install`. Do not edit by hand.\n\
             # Commit it so every machine installs exactly the same versions.\n\n{}",
            body
        );
        fs::write(path, text).map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn get(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}

// ── index.toml ────────────────────────────────────────────────────────────────

// Published versions per package, maintained in src/packages/index.toml.
// A version must be listed here before its tag is pushed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    #[serde(flatten)]
    pub packages: BTreeMap<String, IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    #[serde(default)]
    pub description: String,
    pub versions: Vec<String>,
}

impl Index {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("index.toml: {}", e))
    }

    pub fn latest(&self, name: &str) -> Option<Version> {
        let entry = self.packages.get(name)?;
        highest_matching(&entry.versions, &VersionReq::STAR)
    }
}

// ── Platform naming ───────────────────────────────────────────────────────────

// Platform key used in release asset names: windows-x86_64, linux-x86_64,
// macos-aarch64...
pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

// File name the VM loader expects on this platform for a library name:
// liphia_package_num.dll, libliphia_package_num.so, libliphia_package_num.dylib.
pub fn lib_file_name(lib: &str) -> String {
    format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        lib,
        std::env::consts::DLL_SUFFIX
    )
}

// Release asset name for a library on this platform:
// liphia_package_num-windows-x86_64.dll.
pub fn lib_asset_name(lib: &str) -> String {
    format!("{}-{}{}", lib, platform_key(), std::env::consts::DLL_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_version_is_exact_like_npm() {
        let req = parse_req("1.2.9").unwrap();
        assert!(req.matches(&Version::parse("1.2.9").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn caret_tilde_and_latest() {
        let caret = parse_req("^1.2.0").unwrap();
        assert!(caret.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!caret.matches(&Version::parse("2.0.0").unwrap()));
        let tilde = parse_req("~1.2.0").unwrap();
        assert!(tilde.matches(&Version::parse("1.2.5").unwrap()));
        assert!(!tilde.matches(&Version::parse("1.3.0").unwrap()));
        assert!(parse_req("latest").unwrap().matches(&Version::parse("9.9.9").unwrap()));
    }

    #[test]
    fn highest_matching_picks_the_newest_allowed() {
        let versions: Vec<String> = ["1.0.0", "1.2.9", "1.3.0", "2.0.0"].iter().map(|s| s.to_string()).collect();
        let req = parse_req("^1.0.0").unwrap();
        assert_eq!(highest_matching(&versions, &req), Some(Version::parse("1.3.0").unwrap()));
        let exact = parse_req("1.2.9").unwrap();
        assert_eq!(highest_matching(&versions, &exact), Some(Version::parse("1.2.9").unwrap()));
    }

    #[test]
    fn project_manifest_round_trip() {
        let text = r#"
[package]
name = "demo"

[dependencies]
num = "^1.0.0"
db = { version = "^2.0.0", subpackages = ["sqlite"] }
"#;
        let m: ProjectManifest = toml::from_str(text).unwrap();
        assert_eq!(m.dependencies["num"].req(), "^1.0.0");
        assert_eq!(m.dependencies["db"].subpackages(), ["sqlite".to_string()]);
        let back: ProjectManifest = toml::from_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
        assert_eq!(back.dependencies, m.dependencies);
    }

    #[test]
    fn package_manifest_and_index_parse() {
        let pkg = PackageManifest::parse(
            r#"
[package]
name = "num"
version = "1.0.0"
engine = "=2.0.0"
entry = "num.lph"
files = ["num.lph"]

[native]
abi = "rust-1"
lib = "liphia_package_num"
"#,
        )
        .unwrap();
        assert!(pkg.is_native());
        assert!(pkg.engine_req().unwrap().matches(&Version::parse("2.0.0").unwrap()));
        assert!(!pkg.engine_req().unwrap().matches(&Version::parse("2.0.1").unwrap()));

        let index = Index::parse("[num]\nversions = [\"1.0.0\", \"1.1.0\"]\n").unwrap();
        assert_eq!(index.latest("num"), Some(Version::parse("1.1.0").unwrap()));
    }
}
