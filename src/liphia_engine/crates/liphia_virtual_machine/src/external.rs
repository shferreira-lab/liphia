// liphia_virtual_machine/src/external.rs
//
// Native packages: dynamic loading of prebuilt libraries that are NOT
// compiled into the liphia_cli / liphia_cli_gui binary.
//
// Everything outside liphia_core_native is a package (src/packages/<name>/).
// Pure packages are only .lph files; native packages (db, num, stats,
// learn) also ship a shared library (.so / .dylib / .dll) that this loader
// dlopens when the package is installed.
//
// A native package opts in by shipping an `index.lph` manifest in its
// directory, next to its `<name>.lph` entry file:
//
//   liphia_modules/db/
//     db.lph          Liphia-facing entry (docs + composed functions)
//     module.toml     metadata
//     index.lph       load manifest, read by this loader
//     lib/
//       liphia_package_db.dll   (or libliphia_package_db.so / .dylib)
//
// index.lph is not compiled or executed like a normal .lph source file.
// It is a small line-oriented manifest:
//
//   # comments start with '#', blank lines are ignored
//   external "liphia_package_db"
//
// Each `external "<n>"` line names a library (no prefix/suffix) under
// `<package_dir>/lib/`. The loader resolves the platform file name
// (`lib<n>.so`, `<n>.dll`, `lib<n>.dylib`) via
// `std::env::consts::{DLL_PREFIX, DLL_SUFFIX}`, dlopens it, and calls its
// `liphia_register_module` entry point, a `#[no_mangle] extern "C"
// fn(*mut VM)` that registers the package's natives with
// `VM::register_native`.
//
// The library MUST be built against the same `liphia_virtual_machine`
// version and the same rustc toolchain as the host binary: Rust has no
// stable ABI, so a mismatched build is undefined behavior. This is why the
// package crates live in the same Cargo workspace (src/packages/*/native).
// A C ABI ("c-1") will lift this constraint later.

use std::fs;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use crate::vm::{VmError, VmResult, VM};

/// Symbol every external module library must export.
const ENTRY_SYMBOL: &[u8] = b"liphia_register_module";

/// Symbol returning the library's ABI tag (see `ABI_TAG`).
const ABI_SYMBOL: &[u8] = b"liphia_package_abi";

/// Identifies the binary interface between the VM and native packages:
/// calling convention, VM version and the exact rustc that compiled it.
/// A package library embeds the tag of the VM crate it was compiled
/// against (through `export_package_abi!`), and the loader refuses any
/// library whose tag differs from its own. With the Rust ABI there is no
/// layout guarantee across compilers or VM versions, so equality is the
/// only safe rule.
pub const ABI_TAG: &str = concat!(
    "rust-1;vm=",
    env!("CARGO_PKG_VERSION"),
    ";",
    env!("LIPHIA_RUSTC_VERSION"),
    "\0"
);

/// Returns `ABI_TAG` without its trailing NUL, for messages.
pub fn abi_tag() -> &'static str {
    ABI_TAG.trim_end_matches('\0')
}

/// Exports `liphia_package_abi` from a native package library. Every
/// package crate calls this once at its root:
///
/// ```ignore
/// liphia_virtual_machine::export_package_abi!();
/// ```
#[macro_export]
macro_rules! export_package_abi {
    () => {
        #[no_mangle]
        pub extern "C" fn liphia_package_abi() -> *const ::std::os::raw::c_char {
            $crate::external::ABI_TAG.as_ptr() as *const ::std::os::raw::c_char
        }
    };
}

type AbiFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

/// Signature of the entry point exported by an external module library.
type RegisterFn = unsafe extern "C" fn(*mut VM);

impl VM {
    /// Reads `<module_dir>/index.lph`, resolves each `external "<name>"`
    /// instruction to a prebuilt library under `<module_dir>/lib/`, and
    /// loads + registers all of them into this VM.
    ///
    /// Leaked on purpose: each `Library` is kept alive for the lifetime of
    /// the process (via `Box::leak`) so the function pointers registered
    /// into `native_fns` stay valid. Modules are meant to be loaded once,
    /// at startup — this is not a hot-reload mechanism.
    pub fn load_external_module(&mut self, module_dir: impl AsRef<Path>) -> VmResult<usize> {
        let module_dir = module_dir.as_ref();
        let index_path = module_dir.join("index.lph");
        let manifest = fs::read_to_string(&index_path).map_err(|e| {
            VmError::new(format!(
                "load_external_module: could not read {}: {}",
                index_path.display(),
                e
            ))
        })?;

        let mut loaded = 0usize;
        for name in parse_index(&manifest) {
            let lib_path = resolve_library_path(module_dir, &name);
            self.load_external_library(&lib_path)?;
            loaded += 1;
        }

        if loaded == 0 {
            return Err(VmError::new(format!(
                "load_external_module: {} has no `external \"...\"` instructions",
                index_path.display()
            )));
        }
        Ok(loaded)
    }

    /// Scans `modules_dir` (one subdirectory per installed module, as laid
    /// out by `liphia install <module>` — e.g. `liphia_modules/`) and loads
    /// every subdirectory that has an `index.lph`. Non-external modules
    /// (no `index.lph`) are silently skipped. Returns the names of modules
    /// that failed to load, paired with the error, so the caller can decide
    /// how to surface that (warn and continue, or abort) — a missing or
    /// broken external module should not be fatal to callers that don't
    /// use it. Safe to call even if `modules_dir` does not exist yet
    /// (returns an empty result).
    pub fn load_installed_external_modules(
        &mut self,
        modules_dir: impl AsRef<Path>,
    ) -> Vec<(String, VmError)> {
        let mut failures = vec![];
        let Ok(entries) = fs::read_dir(modules_dir.as_ref()) else {
            return failures;
        };
        for entry in entries.flatten() {
            let module_dir = entry.path();
            if !module_dir.is_dir() || !module_dir.join("index.lph").exists() {
                continue;
            }
            let name = module_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Err(e) = self.load_external_module(&module_dir) {
                failures.push((name, e));
            }
        }
        failures
    }

    /// Loads a single prebuilt native library by path and calls its
    /// `liphia_register_module` entry point against this VM.
    pub fn load_external_library(&mut self, lib_path: impl AsRef<Path>) -> VmResult<()> {
        let lib_path = lib_path.as_ref();
        if !lib_path.exists() {
            return Err(VmError::new(format!(
                "load_external_library: no prebuilt library at {} \
                 (run `liphia install <module>` to fetch it)",
                lib_path.display()
            )));
        }

        // SAFETY: the library is expected to be a Liphia external module
        // built against a matching liphia_virtual_machine version, per the
        // module's own documentation. This is an inherent trust boundary
        // of any Rust plugin-via-dylib system.
        let lib = unsafe { Library::new(lib_path) }.map_err(|e| {
            VmError::new(format!(
                "load_external_library: failed to load {}: {}",
                lib_path.display(),
                e
            ))
        })?;

        check_abi(&lib, lib_path)?;

        // SAFETY: symbol type asserted to match RegisterFn's declared ABI.
        let register: Symbol<RegisterFn> = unsafe { lib.get(ENTRY_SYMBOL) }.map_err(|e| {
            VmError::new(format!(
                "load_external_library: {} does not export `{}`: {}",
                lib_path.display(),
                String::from_utf8_lossy(ENTRY_SYMBOL),
                e
            ))
        })?;

        // SAFETY: `self` is a valid, non-null `&mut VM` for the duration of
        // this call; the callee is expected only to call
        // `VM::register_native` on it, per the external-module contract.
        unsafe { register(self as *mut VM) };

        // Keep the library mapped for the rest of the process's life so
        // the fn pointers we just registered remain valid.
        Box::leak(Box::new(lib));

        Ok(())
    }
}

/// Reads the library's ABI tag and compares it with this VM's. Runs before
/// any other function of the library is called, so a mismatched build is
/// reported as an error instead of causing undefined behavior.
fn check_abi(lib: &Library, lib_path: &Path) -> VmResult<()> {
    // SAFETY: `liphia_package_abi` takes no arguments and returns a pointer
    // to a NUL-terminated static string (see export_package_abi!).
    let abi: Symbol<AbiFn> = unsafe { lib.get(ABI_SYMBOL) }.map_err(|_| {
        VmError::new(format!(
            "{} was built for an older engine (no ABI tag); reinstall the package \
             for this engine ({})",
            lib_path.display(),
            abi_tag()
        ))
    })?;
    // SAFETY: see above; the pointer is valid for the library's lifetime.
    let theirs = unsafe { std::ffi::CStr::from_ptr(abi()) }
        .to_string_lossy()
        .to_string();
    if theirs != abi_tag() {
        return Err(VmError::new(format!(
            "{} was built for a different engine\n  package: {}\n  engine:  {}\n  \
             reinstall a package version built for this engine",
            lib_path.display(),
            theirs,
            abi_tag()
        )));
    }
    Ok(())
}

/// Parses `external "<name>"` instructions out of an index.lph manifest.
/// Ignores blank lines and `#` comments; unrecognized lines are skipped
/// rather than treated as errors, so a manifest can grow new instruction
/// kinds later without breaking older loaders.
fn parse_index(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix("external"))
        .filter_map(|rest| {
            let rest = rest.trim();
            let rest = rest.strip_prefix('"')?;
            let name = rest.strip_suffix('"').unwrap_or(rest.trim_end_matches('"'));
            let name = rest.split('"').next().unwrap_or(name);
            if name.is_empty() { None } else { Some(name.to_string()) }
        })
        .collect()
}

/// Resolves a library base name (as written in index.lph, no prefix/suffix)
/// to `<module_dir>/lib/<platform-specific file name>`.
fn resolve_library_path(module_dir: &Path, name: &str) -> PathBuf {
    let file_name = format!(
        "{}{}{}",
        std::env::consts::DLL_PREFIX,
        name,
        std::env::consts::DLL_SUFFIX
    );
    module_dir.join("lib").join(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_external_instructions() {
        let manifest = r#"
            # comment
            external "liphia_module_db"

            external "another_module"
        "#;
        assert_eq!(
            parse_index(manifest),
            vec!["liphia_module_db".to_string(), "another_module".to_string()]
        );
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        assert_eq!(parse_index("# only a comment\n\n"), Vec::<String>::new());
    }

    #[test]
    fn resolves_platform_file_name() {
        let path = resolve_library_path(Path::new("/mods/db"), "liphia_module_db");
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(
            file_name,
            format!(
                "{}liphia_module_db{}",
                std::env::consts::DLL_PREFIX,
                std::env::consts::DLL_SUFFIX
            )
        );
    }
}
