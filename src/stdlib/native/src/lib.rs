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


pub mod math;
pub mod stats;
pub mod fs;
pub mod net;
pub mod http;
pub mod ws;
pub mod ai;

use liphia_virtual_machine::vm::VM;

/// Registers all standard library modules into the VM.
/// Call this once before `vm.run()`.
/// Stub modules register their functions but return errors if called.
pub fn register_all(vm: &mut VM) {
    math::register(vm);
    stats::register(vm);
    fs::register(vm);
    net::register(vm);
    http::register(vm);
    ws::register(vm);
    ai::register(vm);
}
