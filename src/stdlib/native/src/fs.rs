// stdlib/native/src/fs.rs
//
// File system functions.
//
// Functions registered:
//   read_file(path)              → str     read entire file as string
//   write_file(path, content)    → bool    write string to file, returns true on success
//   append_file(path, content)   → bool    append string to file
//   file_exists(path)            → bool    returns true if file exists

use std::rc::Rc;
use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

pub fn register(vm: &mut VM) {
    vm.register_native("read_file",   native_read_file);
    vm.register_native("write_file",  native_write_file);
    vm.register_native("append_file", native_append_file);
    vm.register_native("file_exists", native_file_exists);
}

fn native_read_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("read_file", &args, 1)?;
    match &args[0] {
        Value::Str(path) => {
            std::fs::read_to_string(path.as_str())
                .map(|s| Value::Str(Rc::new(s)))
                .map_err(|e| VmError::new(format!("read_file(): {}", e)))
        }
        _ => Err(VmError::new("read_file(): path must be a str")),
    }
}

fn native_write_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("write_file", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(path), Value::Str(content)) => {
            std::fs::write(path.as_str(), content.as_str())
                .map(|_| Value::Bool(true))
                .map_err(|e| VmError::new(format!("write_file(): {}", e)))
        }
        _ => Err(VmError::new("write_file(): path and content must be str")),
    }
}

fn native_append_file(args: Vec<Value>) -> VmResult<Value> {
    expect_args("append_file", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(path), Value::Str(content)) => {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path.as_str())
                .map_err(|e| VmError::new(format!("append_file(): {}", e)))?;
            file.write_all(content.as_bytes())
                .map(|_| Value::Bool(true))
                .map_err(|e| VmError::new(format!("append_file(): {}", e)))
        }
        _ => Err(VmError::new("append_file(): path and content must be str")),
    }
}

fn native_file_exists(args: Vec<Value>) -> VmResult<Value> {
    expect_args("file_exists", &args, 1)?;
    match &args[0] {
        Value::Str(path) => Ok(Value::Bool(std::path::Path::new(path.as_str()).exists())),
        _ => Err(VmError::new("file_exists(): path must be a str")),
    }
}

fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}", name, expected, args.len()
        )))
    } else { Ok(()) }
}
