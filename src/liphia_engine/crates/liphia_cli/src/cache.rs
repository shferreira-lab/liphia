// liphia_cli/src/cache.rs
//
// On-disk bytecode cache: liphia_cache/<stem>.lbc next to the source file.
// The file format itself belongs to liphia_bytecode; this module only
// decides where the file lives and when it is still valid.

use std::fs;
use std::path::{Path, PathBuf};

use liphia_virtual_machine::opcode::Opcode;

pub fn cache_path(source_path: &Path) -> PathBuf {
    let dir = source_path.parent().unwrap_or(Path::new("."));
    let stem = source_path.file_stem().unwrap_or_default().to_string_lossy();
    dir.join("liphia_cache").join(format!("{}.lbc", stem))
}

// FNV-1a 64-bit over the resolved AST's debug text. Any source change
// (including imported files) changes the hash and invalidates the cache.
pub fn source_hash(source: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in source.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn load_cache(source_path: &Path, current_hash: u64) -> Option<Vec<Opcode>> {
    let path = cache_path(source_path);
    let data = fs::read(&path).ok()?;

    // Cheap header check first: a stale file is the common case.
    match liphia_bytecode::peek_source_hash(&data) {
        Ok(h) if h == current_hash => {}
        Ok(_) => return None,
        Err(e) => {
            eprintln!("[cache] ignoring {}: {}", path.display(), e);
            return None;
        }
    }

    match liphia_bytecode::decode(&data) {
        Ok(program) => Some(program.opcodes),
        Err(e) => {
            eprintln!("[cache] ignoring {}: {}", path.display(), e);
            None
        }
    }
}

pub fn save_cache(source_path: &Path, hash: u64, opcodes: &[Opcode]) {
    let path = cache_path(source_path);
    if let Some(dir) = path.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("[cache] could not create liphia_cache/: {}", e);
            return;
        }
    }
    if let Err(e) = fs::write(&path, liphia_bytecode::encode(hash, opcodes)) {
        eprintln!("[cache] could not write {}: {}", path.display(), e);
    }
}
