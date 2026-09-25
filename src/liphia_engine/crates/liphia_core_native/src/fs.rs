// liphia_core_native/src/fs.rs
//
// File system.
//
// Functions registered:
//   read_file(path: str)                    -> str    whole file as text
//   write_file(path: str, content: str)     -> bool   create or overwrite
//   append_file(path: str, content: str)    -> bool   create or append
//   file_exists(path: str)                  -> bool
//
//   read_json(path: str)                    -> any    read_file + json_decode
//   write_json(path: str, data: any)        -> bool   json_encode + write_file
//   append_json_line(path: str, data: any)  -> bool   one JSON document per line (JSON Lines)
//
// I/O failures are errors (catchable with try/catch); the bool results are
// always true when the call returns.

use std::io::Write;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::json;
use crate::util::{expect_args, str_arg, str_value};

pub fn register(vm: &mut VM) {
    vm.register_native("read_file", native_read_file);
    vm.register_native("write_file", native_write_file);
    vm.register_native("append_file", native_append_file);
    vm.register_native("file_exists", native_file_exists);
    vm.register_native("read_json", native_read_json);
    vm.register_native("write_json", native_write_json);
    vm.register_native("append_json_line", native_append_json_line);
}

// ── Shared operations ─────────────────────────────────────────────────────────

fn read(name: &str, path: &str) -> VmResult<String> {
    std::fs::read_to_string(path).map_err(|e| VmError::new(format!("{}(): {}: {}", name, path, e)))
}

fn write(name: &str, path: &str, content: &str) -> VmResult<Value> {
    std::fs::write(path, content)
        .map(|_| Value::Bool(true))
        .map_err(|e| VmError::new(format!("{}(): {}: {}", name, path, e)))
}

fn append(name: &str, path: &str, content: &str) -> VmResult<Value> {
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .and_then(|mut file| file.write_all(content.as_bytes()))
        .map(|_| Value::Bool(true))
        .map_err(|e| VmError::new(format!("{}(): {}: {}", name, path, e)))
}

// ── Text files ────────────────────────────────────────────────────────────────

fn native_read_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("read_file", &args, 1)?;
    let path = str_arg(&args[0], "read_file")?;
    Ok(str_value(read("read_file", &path)?))
}

fn native_write_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("write_file", &args, 2)?;
    let path = str_arg(&args[0], "write_file")?;
    let content = str_arg(&args[1], "write_file")?;
    write("write_file", &path, &content)
}

fn native_append_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("append_file", &args, 2)?;
    let path = str_arg(&args[0], "append_file")?;
    let content = str_arg(&args[1], "append_file")?;
    append("append_file", &path, &content)
}

fn native_file_exists(args: Vec<Value>) -> VmResult<Value> {
    expect_args("file_exists", &args, 1)?;
    let path = str_arg(&args[0], "file_exists")?;
    Ok(Value::Bool(std::path::Path::new(&path).exists()))
}

// ── JSON files ────────────────────────────────────────────────────────────────

fn native_read_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("read_json", &args, 1)?;
    let path = str_arg(&args[0], "read_json")?;
    let text = read("read_json", &path)?;
    json::decode(&text).map_err(|e| VmError::new(format!("read_json(): {}: {}", path, e)))
}

fn native_write_json(args: Vec<Value>) -> VmResult<Value> {
    expect_args("write_json", &args, 2)?;
    let path = str_arg(&args[0], "write_json")?;
    write("write_json", &path, &json::encode(&args[1]))
}

fn native_append_json_line(args: Vec<Value>) -> VmResult<Value> {
    expect_args("append_json_line", &args, 2)?;
    let path = str_arg(&args[0], "append_json_line")?;
    let line = format!("{}\n", json::encode(&args[1]));
    append("append_json_line", &path, &line)
}
