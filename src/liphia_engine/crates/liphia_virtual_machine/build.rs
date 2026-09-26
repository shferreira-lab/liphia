// liphia_virtual_machine/build.rs
//
// Records the rustc version that compiles this crate. It becomes part of
// external::ABI_TAG, so a native package built by a different compiler is
// rejected at load time instead of running with a mismatched Rust ABI.

use std::env;
use std::process::Command;

fn main() {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(rustc)
        .arg("-V")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "rustc-unknown".to_string());
    println!("cargo:rustc-env=LIPHIA_RUSTC_VERSION={}", version);
    println!("cargo:rerun-if-env-changed=RUSTC");
}
