// stdlib/native/src/net.rs
//
// TCP networking — blocking I/O using std::net.
// Connections are stored in a thread_local registry keyed by handle (i64).
//
// Functions registered:
//   tcp_connect(host: str, port: int) -> int    open connection, returns handle
//   tcp_send(handle: int, data: str)  -> bool
//   tcp_recv(handle: int)             -> str     reads available bytes (up to 4096)
//   tcp_close(handle: int)            -> bool
//   udp_send(host: str, port: int, data: str) -> bool

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::rc::Rc;

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

// Thread-local connection registry
thread_local! {
    static CONNECTIONS: RefCell<HashMap<i64, TcpStream>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: RefCell<i64> = RefCell::new(1);
}

pub fn register(vm: &mut VM) {
    vm.register_native("tcp_connect", native_tcp_connect);
    vm.register_native("tcp_send",    native_tcp_send);
    vm.register_native("tcp_recv",    native_tcp_recv);
    vm.register_native("tcp_close",   native_tcp_close);
    vm.register_native("udp_send",    native_udp_send);
}

fn native_tcp_connect(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 {
        return Err(VmError::new("tcp_connect(host: str, port: int) — expected 2 arguments"));
    }
    let host = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("tcp_connect: host must be str")),
    };
    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        _ => return Err(VmError::new("tcp_connect: port must be int")),
    };

    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)
        .map_err(|e| VmError::new(format!("tcp_connect: failed to connect to {}: {}", addr, e)))?;

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
    if args.len() != 2 {
        return Err(VmError::new("tcp_send(handle: int, data: str) — expected 2 arguments"));
    }
    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("tcp_send: handle must be int")),
    };
    let data = match &args[1] {
        Value::Str(s) => s.as_bytes().to_vec(),
        _ => return Err(VmError::new("tcp_send: data must be str")),
    };

    CONNECTIONS.with(|c| {
        let mut map = c.borrow_mut();
        let stream = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("tcp_send: no connection with handle {}", handle)))?;
        stream.write_all(&data)
            .map(|_| Value::Bool(true))
            .map_err(|e| VmError::new(format!("tcp_send: {}", e)))
    })
}

fn native_tcp_recv(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("tcp_recv(handle: int) — expected 1 argument"));
    }
    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("tcp_recv: handle must be int")),
    };

    CONNECTIONS.with(|c| {
        let mut map = c.borrow_mut();
        let stream = map.get_mut(&handle)
            .ok_or_else(|| VmError::new(format!("tcp_recv: no connection with handle {}", handle)))?;
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf)
            .map_err(|e| VmError::new(format!("tcp_recv: {}", e)))?;
        let text = String::from_utf8_lossy(&buf[..n]).to_string();
        Ok(Value::Str(Rc::new(text)))
    })
}

fn native_tcp_close(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("tcp_close(handle: int) — expected 1 argument"));
    }
    let handle = match &args[0] {
        Value::Int(h) => *h,
        _ => return Err(VmError::new("tcp_close: handle must be int")),
    };
    CONNECTIONS.with(|c| {
        let removed = c.borrow_mut().remove(&handle).is_some();
        Ok(Value::Bool(removed))
    })
}

fn native_udp_send(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 3 {
        return Err(VmError::new("udp_send(host: str, port: int, data: str) — expected 3 arguments"));
    }
    let host = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::new("udp_send: host must be str")),
    };
    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        _ => return Err(VmError::new("udp_send: port must be int")),
    };
    let data = match &args[2] {
        Value::Str(s) => s.as_bytes().to_vec(),
        _ => return Err(VmError::new("udp_send: data must be str")),
    };

    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| VmError::new(format!("udp_send: failed to bind socket: {}", e)))?;
    let dest = format!("{}:{}", host, port);
    socket.send_to(&data, &dest)
        .map(|_| Value::Bool(true))
        .map_err(|e| VmError::new(format!("udp_send: {}", e)))
}
