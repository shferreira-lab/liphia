pub mod core;

use liphia_virtual_machine::vm::VM;

pub fn register(vm: &mut VM) {
    core::register(vm);
}