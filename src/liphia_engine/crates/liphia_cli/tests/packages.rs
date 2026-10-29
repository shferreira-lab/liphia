// liphia_cli/tests/packages.rs
//
// Package conformance suite. Every folder conformance/cases/<package>/
// holds the cases of one official package (crypto/, num/, ...), written
// exactly like the language cases:
//
//   <case>.lph  — the program
//   <case>.out  — expected stdout, exact match, exit code 0
//   <case>.err  — expected failure: non-zero exit and output containing
//                 this text (compile or runtime error)
//
// Unlike the language cases, these programs need the package installed, and
// the VM only loads a native package from ./liphia_modules/ of its working
// directory. So for each folder this test:
//
//   1. builds the native library of the package and of its dependencies
//      (cargo build -p <lib>, same profile and toolchain as this test);
//   2. installs them into a scratch directory, laid out like `liphia
//      install` does: liphia_modules/<name>/ with the package files and
//      lib/ holding the library;
//   3. runs every case with the scratch directory as working directory.
//
// The language suite (tests/conformance.rs) does not look inside these
// folders, so it stays VM-agnostic: a package suite needs this engine's
// native loader and Rust ABI.
//
// Run with: cargo test -p liphia_cli --test packages

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// Folders of conformance/cases that are not package suites.
const HELPER_DIRS: &[&str] = &["modules"];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn cases_dir() -> PathBuf {
    src_dir().join("conformance/cases")
}

fn packages_dir() -> PathBuf {
    src_dir().join("packages")
}

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end().to_string()
}

// ── package.toml (just the two things this test needs) ───────────────────────

// Value of `key` in the first `[section]` that has it, as a quoted string.
fn toml_string(toml: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            inside = &line[1..line.len() - 1] == section;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

// Names declared under `[dependencies]`.
fn dependencies(toml: &str) -> Vec<String> {
    let mut inside = false;
    let mut names = vec![];
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            inside = &line[1..line.len() - 1] == "dependencies";
            continue;
        }
        if inside && !line.starts_with('#') {
            if let Some((name, _)) = line.split_once('=') {
                names.push(name.trim().to_string());
            }
        }
    }
    names
}

// ── Build and install ─────────────────────────────────────────────────────────

// Directory of the liphia binary under test (target/<profile>/). A package
// library built with the same profile lands next to it.
fn profile_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_liphia"))
        .parent()
        .expect("liphia binary has a parent directory")
        .to_path_buf()
}

fn build_library(lib: &str) {
    let release = profile_dir().file_name().map_or(false, |n| n == "release");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(cargo);
    command.args(["build", "-p", lib]);
    if release {
        command.arg("--release");
    }
    let output = command
        .current_dir(src_dir())
        .output()
        .expect("failed to start cargo");
    assert!(
        output.status.success(),
        "cargo build -p {} failed:\n{}",
        lib,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn copy_package_files(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create package folder");
    for entry in fs::read_dir(from).expect("read package folder").flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Rust sources and local builds are not part of an installed package.
        if name == "native" || name == "lib" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy_package_files(&path, &to.join(&name));
        } else {
            fs::copy(&path, to.join(&name)).expect("copy package file");
        }
    }
}

// Installs `name` and, first, everything it depends on.
fn install(name: &str, modules: &Path, done: &mut HashSet<String>) {
    if !done.insert(name.to_string()) {
        return;
    }
    let from = packages_dir().join(name);
    let toml = fs::read_to_string(from.join("package.toml"))
        .unwrap_or_else(|_| panic!("no package '{}' (missing {}/package.toml)", name, from.display()));
    for dependency in dependencies(&toml) {
        install(&dependency, modules, done);
    }

    let to = modules.join(name);
    copy_package_files(&from, &to);

    if let Some(lib) = toml_string(&toml, "native", "lib") {
        build_library(&lib);
        let file = format!(
            "{}{}{}",
            std::env::consts::DLL_PREFIX,
            lib,
            std::env::consts::DLL_SUFFIX
        );
        let built = profile_dir().join(&file);
        let lib_dir = to.join("lib");
        fs::create_dir_all(&lib_dir).expect("create lib folder");
        fs::copy(&built, lib_dir.join(&file))
            .unwrap_or_else(|e| panic!("could not copy {}: {}", built.display(), e));
    }
}

// ── Running cases ─────────────────────────────────────────────────────────────

// Outcome of one case: None if it passed, Some(reason) if it failed.
fn run_case(source: &Path, workdir: &Path) -> Option<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_liphia"))
        .arg(source)
        .arg("--no-cache")
        .current_dir(workdir)
        .stdin(Stdio::null())
        .output()
        .expect("failed to start liphia_cli");

    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    let stderr = normalize(&String::from_utf8_lossy(&output.stderr));
    let expected_out = source.with_extension("out");
    let expected_err = source.with_extension("err");

    if expected_out.exists() {
        let expected = normalize(&fs::read_to_string(&expected_out).unwrap());
        if !output.status.success() {
            return Some(format!("exited with {}\n{}", output.status, stderr));
        }
        if stdout != expected {
            return Some(format!("stdout mismatch\n--- expected\n{}\n--- got\n{}", expected, stdout));
        }
        return None;
    }

    if expected_err.exists() {
        let needle = normalize(&fs::read_to_string(&expected_err).unwrap());
        if output.status.success() {
            return Some("expected a failure, but exited with 0".to_string());
        }
        let combined = format!("{}\n{}", stdout, stderr);
        if !combined.contains(&needle) {
            return Some(format!("error text '{}' not found in:\n{}", needle, combined));
        }
        return None;
    }

    Some("missing .out or .err file".to_string())
}

// The package folders of conformance/cases, sorted by name.
fn suites() -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = fs::read_dir(cases_dir())
        .expect("conformance/cases not found")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
        .filter(|(name, _)| !HELPER_DIRS.contains(&name.as_str()))
        .collect();
    found.sort();
    found
}

#[test]
fn package_conformance() {
    let suites = suites();
    assert!(!suites.is_empty(), "no package suites found in conformance/cases");

    let mut failures: Vec<String> = vec![];
    let mut total = 0;

    for (package, dir) in &suites {
        let mut sources: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map_or(false, |x| x == "lph"))
            .collect();
        sources.sort();
        assert!(!sources.is_empty(), "conformance/cases/{} has no cases", package);

        // A fresh scratch directory per suite, removed at the end.
        let scratch = std::env::temp_dir()
            .join(format!("liphia-package-suite-{}-{}", package, std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        fs::create_dir_all(&scratch).expect("create scratch directory");
        install(package, &scratch.join("liphia_modules"), &mut HashSet::new());

        for source in &sources {
            total += 1;
            let name = source.file_name().unwrap().to_string_lossy().to_string();
            if let Some(reason) = run_case(source, &scratch) {
                failures.push(format!("[{}/{}] {}", package, name, reason));
            }
        }
        let _ = fs::remove_dir_all(&scratch);
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} package conformance case(s) failed:\n\n{}",
            failures.len(),
            total,
            failures.join("\n\n")
        );
    }
}
