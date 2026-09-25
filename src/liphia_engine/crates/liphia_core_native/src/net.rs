// liphia_core_native/src/net.rs
//
// TCP client and UDP send, blocking I/O on std::net. Connections live in a
// thread-local registry keyed by an int handle.
//
// Functions registered:
//   tcp_connect(host: str, port: int)         -> int    connection handle
//   tcp_send(handle: int, data: str)          -> bool
//   tcp_recv(handle: int)                     -> str    up to 4096 bytes; "" when the peer closed
//   tcp_recv_all(handle: int)                 -> str    reads until the peer closes
//   tcp_close(handle: int)                    -> bool   false if the handle was unknown
//   udp_send(host: str, port: int, data: str) -> bool
//
//   tcp_send_json(handle: int, data: any)     -> bool   json_encode + tcp_send
//   tcp_recv_json(handle: int)                -> any    tcp_recv_all + json_decode
//
// tcp_recv blocks until at least one byte arrives or the peer closes.
// tcp_recv_all and tcp_recv_json block until the peer closes the
// connection, so they fit request/response protocols where the server
// closes after answering.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::json;
use crate::util::{expect_args, int_arg, port_arg, str_arg, str_value};

thread_local! {
    static CONNECTIONS: RefCell<HashMap<i64, TcpStream>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: RefCell<i64> = RefCell::new(1);
}

pub fn register(vm: &mut VM) {
    vm.register_native("tcp_connect", native_tcp_connect);
    vm.register_native("tcp_send", native_tcp_send);
    vm.register_native("tcp_recv", native_tcp_recv);
    vm.register_native("tcp_recv_all", native_tcp_recv_all);
    vm.register_native("tcp_close", native_tcp_close);
    vm.register_native("udp_send", native_udp_send);
    vm.register_native("tcp_send_json", native_tcp_send_json);
    vm.register_native("tcp_recv_json", native_tcp_recv_json);
}

// ── Registry operations ───────────────────────────────────────────────────────

fn with_stream<T>(
    name: &str,
    handle: i64,
    f: impl FnOnce(&mut TcpStream) -> std::io::Result<T>,
) -> VmResult<T> {
    CONNECTIONS.with(|c| {
        let mut map = c.borrow_mut();
        let stream = map
            .get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("{}(): no connection with handle {}", name, handle)))?;
        f(stream).map_err(|e| VmError::new(format!("{}(): {}", name, e)))
    })
}

fn send(name: &str, handle: i64, data: &[u8]) -> VmResult<Value> {
    with_stream(name, handle, |s| s.write_all(data))?;
    Ok(Value::Bool(true))
}

fn recv_all(name: &str, handle: i64) -> VmResult<String> {
    let bytes = with_stream(name, handle, |s| {
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).map(|_| buf)
    })?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_tcp_connect(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_connect", &args, 2)?;
    let host = str_arg(&args[0], "tcp_connect")?;
    let port = port_arg(&args[1], "tcp_connect")?;
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)
        .map_err(|e| VmError::new(format!("tcp_connect(): {}: {}", addr, e)))?;
    let handle = NEXT_HANDLE.with(|h| {
        let mut h = h.borrow_mut();
        let id = *h;
        *h += 1;
        id
    });
    CONNECTIONS.with(|c| c.borrow_mut().insert(handle, stream));
    Ok(Value::Int(handle))
}

fn native_tcp_send(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_send", &args, 2)?;
    let handle = int_arg(&args[0], "tcp_send")?;
    let data = str_arg(&args[1], "tcp_send")?;
    send("tcp_send", handle, data.as_bytes())
}

fn native_tcp_recv(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_recv", &args, 1)?;
    let handle = int_arg(&args[0], "tcp_recv")?;
    let bytes = with_stream("tcp_recv", handle, |s| {
        let mut buf = vec![0u8; 4096];
        let n = s.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    })?;
    Ok(str_value(String::from_utf8_lossy(&bytes).to_string()))
}

fn native_tcp_recv_all(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_recv_all", &args, 1)?;
    let handle = int_arg(&args[0], "tcp_recv_all")?;
    Ok(str_value(recv_all("tcp_recv_all", handle)?))
}

fn native_tcp_close(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_close", &args, 1)?;
    let handle = int_arg(&args[0], "tcp_close")?;
    let removed = CONNECTIONS.with(|c| c.borrow_mut().remove(&handle).is_some());
    Ok(Value::Bool(removed))
}

fn native_udp_send(args: Vec<Value>) -> VmResult<Value> {
    expect_args("udp_send", &args, 3)?;
    let host = str_arg(&args[0], "udp_send")?;
    let port = port_arg(&args[1], "udp_send")?;
    let data = str_arg(&args[2], "udp_send")?;
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| VmError::new(format!("udp_send(): failed to bind socket: {}", e)))?;
    let dest = format!("{}:{}", host, port);
    socket
        .send_to(data.as_bytes(), &dest)
        .map(|_| Value::Bool(true))
        .map_err(|e| VmError::new(format!("udp_send(): {}: {}", dest, e)))
}

// ── JSON over TCP ─────────────────────────────────────────────────────────────

fn native_tcp_send_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_send_json", &args, 2)?;
    let handle = int_arg(&args[0], "tcp_send_json")?;
    send("tcp_send_json", handle, json::encode(&args[1]).as_bytes())
}

fn native_tcp_recv_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tcp_recv_json", &args, 1)?;
    let handle = int_arg(&args[0], "tcp_recv_json")?;
    let text = recv_all("tcp_recv_json", handle)?;
    json::decode(&text).map_err(|e| VmError::new(format!("tcp_recv_json(): {}", e)))
}
