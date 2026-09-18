// stdlib/native/src/lib.rs
//
// liphia_stdlib_native — native function registry for the Liphia VM.
//
// Usage from liphia_cli/src/main.rs:
//
//   use liphia_stdlib_native;
//   let mut vm = VM::new();
//   liphia_stdlib_native::register_all(&mut vm);
//   vm.run(opcodes)?;
//
// `db` is NOT registered here. It ships as an external module — a prebuilt
// dynamic library under stdlib/modules/db/lib/, loaded on demand via
// `vm.load_external_module("liphia_modules/db")` (see
// liphia_virtual_machine::external and stdlib/modules/db/index.lph) instead
// of being compiled into every liphia_cli / liphia_cli_gui binary. Its
// source now lives in liphia_engine/crates/liphia_module_db.

mod cdf;
pub mod ai;
pub mod fs;
pub mod http;
pub mod json;
pub mod math;
pub mod net;
pub mod stats;
pub mod ws;


use liphia_virtual_machine::vm::VM;

/// Registers the standard library modules that are always compiled into
/// the binary. Call this once before `vm.run()`. `db` is excluded — load
/// it separately with `vm.load_external_module(...)` if the project needs it.
pub fn register_all(vm: &mut VM) {
    ai::register(vm);
    fs::register(vm);
    http::register(vm);
    json::register(vm);
    math::register(vm);
    net::register(vm);
    stats::register(vm);
    ws::register(vm);
}