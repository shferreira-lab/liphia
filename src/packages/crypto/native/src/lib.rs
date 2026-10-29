// packages/crypto/native/src/lib.rs
//
// Native library of the `crypto` package: cryptographic primitives for
// Liphia programs. Loaded by the VM through
// liphia_virtual_machine::external (see packages/crypto/index.lph).
//
// Every function is registered globally under a `crypto_` prefix, because
// package natives share one namespace with the core and with each other.
//
// Binary data: the VM has no bytes type yet, so binary values (digests,
// keys, nonces) are lowercase hex strings. Functions that take binary input
// have a `_hex` variant that accepts hex and decodes it first.

mod hash;
mod hex;
mod util;

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
    hash::register(vm);
}
