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

mod cdf;
pub mod ai;
pub mod db;
pub mod fs;
pub mod http;
pub mod json;
pub mod math;
pub mod net;
pub mod stats;
pub mod ws;


use liphia_virtual_machine::vm::VM;

/// Registers all standard library modules into the VM.
/// Call this once before `vm.run()`.
pub fn register_all(vm: &mut VM) {
    ai::register(vm);
    db::register(vm);
    fs::register(vm);
    http::register(vm);
    json::register(vm);
    math::register(vm);
    net::register(vm);
    stats::register(vm);
    ws::register(vm);
}