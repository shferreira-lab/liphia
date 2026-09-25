// liphia_core_native/src/util.rs
//
// Argument helpers shared by every core native module. Each native receives
// its arguments as a Vec<Value>; these helpers validate arity and extract
// typed values with consistent error messages ("name(): ...").

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult};

pub fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        return Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}",
            name,
            expected,
            args.len()
        )));
    }
    Ok(())
}

// Numeric argument as f64. Int is accepted because math functions take
// "a number" by contract; this is conversion by the function, not operator
// coercion.
pub fn num_arg(value: &Value, name: &str) -> VmResult<f64> {
    match value {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(VmError::new(format!("{}(): argument must be int or float", name))),
    }
}

pub fn int_arg(value: &Value, name: &str) -> VmResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        _ => Err(VmError::new(format!("{}(): argument must be int", name))),
    }
}

pub fn str_arg(value: &Value, name: &str) -> VmResult<String> {
    match value {
        Value::Str(s) => Ok(s.as_str().to_string()),
        _ => Err(VmError::new(format!("{}(): argument must be str", name))),
    }
}

pub fn str_value(text: impl Into<String>) -> Value {
    Value::Str(Rc::new(text.into()))
}

pub fn list_value(items: Vec<Value>) -> Value {
    Value::List(Rc::new(RefCell::new(items)))
}

// TCP/UDP port: an int in 1..=65535. Out-of-range values are an error
// instead of being truncated to u16.
pub fn port_arg(value: &Value, name: &str) -> VmResult<u16> {
    let port = int_arg(value, name)?;
    if !(1..=65535).contains(&port) {
        return Err(VmError::new(format!("{}(): port must be in 1..65535", name)));
    }
    Ok(port as u16)
}
