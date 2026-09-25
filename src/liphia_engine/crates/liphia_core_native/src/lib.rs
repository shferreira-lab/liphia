// liphia_core_native/src/lib.rs
//
// Core natives: every function that is compiled into the Liphia binary and
// callable without any import. Anything outside this crate is a package
// (src/packages/<name>/), installed with `liphia install <name>`.
//
// Modules:
//   core    strings, lists, maps, conversions
//   math    scalar math
//   random  pseudo-random numbers
//   agg     sum, mean, min_list, max_list
//   json    json_encode, json_decode
//   fs      files (+ read_json, write_json, append_json_line)
//   net     tcp/udp (+ tcp_recv_all, tcp_send_json, tcp_recv_json)
//   http    http server and client
//   ws      websocket server (+ ws_send_json, ws_broadcast_json)
//
// The full reference lives in docs/core.md; the compiler's type signatures
// for these natives live in liphia_compiler::type_checker. Adding a native
// means touching all three.

pub mod agg;
pub mod core;
pub mod fs;
pub mod http;
pub mod json;
pub mod math;
pub mod net;
pub mod random;
pub mod ws;

mod util;

use liphia_virtual_machine::vm::VM;

pub fn register(vm: &mut VM) {
    core::register(vm);
    math::register(vm);
    random::register(vm);
    agg::register(vm);
    json::register(vm);
    fs::register(vm);
    net::register(vm);
    http::register(vm);
    ws::register(vm);
}
