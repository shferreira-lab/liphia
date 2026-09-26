// liphia_cli/tests/conformance.rs
//
// Language conformance suite. Runs every conformance/cases/*.lph through
// the real liphia_cli binary and compares the result with a sibling file:
//
//   <case>.out  — expected stdout, exact match, exit code 0
//   <case>.err  — expected failure: non-zero exit and output containing
//                 this text (compile or runtime error)
//
// The cases are VM-agnostic: any future Liphia VM must pass the same set.
// Run with: cargo test -p liphia_cli --test conformance

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../conformance/cases")
}

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end().to_string()
}

// Outcome of one case: None if it passed, Some(reason) if it failed.
fn run_case(source: &Path) -> Option<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_liphia"))
        .arg(source)
        .arg("--no-cache")
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

#[test]
fn conformance() {
    let mut sources: Vec<PathBuf> = fs::read_dir(cases_dir())
        .expect("conformance/cases not found")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map_or(false, |x| x == "lph"))
        .collect();
    sources.sort();
    assert!(!sources.is_empty(), "no conformance cases found");

    // Run everything first, then report all failures together.
    let failures: Vec<String> = sources
        .iter()
        .filter_map(|src| {
            let name = src.file_name().unwrap().to_string_lossy().to_string();
            run_case(src).map(|reason| format!("[{}] {}", name, reason))
        })
        .collect();

    if !failures.is_empty() {
        panic!(
            "{} of {} conformance case(s) failed:\n\n{}",
            failures.len(),
            sources.len(),
            failures.join("\n\n")
        );
    }
}
