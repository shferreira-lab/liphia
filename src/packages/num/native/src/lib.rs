// packages/num/native/src/lib.rs
//
// Native library of the `num` package: vectors, matrices and descriptive
// statistics over lists. Loaded by the VM through
// liphia_virtual_machine::external (see packages/num/index.lph).

mod describe;
mod vector;

use liphia_virtual_machine::vm::VM;

// ABI tag checked by the VM loader before liphia_register_module is
// called; it embeds the VM version and rustc this library was built with.
liphia_virtual_machine::export_package_abi!();

// Entry point called by the loader right after dlopen. The signature must
// match liphia_virtual_machine::external's RegisterFn exactly.
#[no_mangle]
pub extern "C" fn liphia_register_module(vm: *mut VM) {
    // SAFETY: the loader passes a valid, non-null *mut VM for the duration
    // of this call.
    let vm = unsafe { &mut *vm };
    vector::register(vm);
    describe::register(vm);
}
