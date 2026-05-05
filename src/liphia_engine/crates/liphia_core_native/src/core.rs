// liphia_core_native/src/core.rs
//
// Core string, list, and value utilities.
//
// Functions registered:
//   len(value)                    → int      length of str or list
//   to_int(value)                 → int      parse str/float to int
//   to_float(value)               → float    parse str/int to float
//   to_str(value)                 → str      convert any value to str
//   trim(str)                     → str      remove leading/trailing whitespace
//   upper(str)                    → str      convert to uppercase
//   lower(str)                    → str      convert to lowercase
//   contains(str, substr)         → bool     true if str contains substr
//   starts_with(str, prefix)      → bool
//   ends_with(str, suffix)        → bool
//   replace(str, from, to)        → str      replace all occurrences
//   split(str, sep)               → list     split string by separator
//   append(list, value)           → null     add element to end of list (in-place)
//   pop(list)                     → any      remove and return last element
//   keys(list)                    → list     returns list of integer indices [0, 1, 2, ...]

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

pub fn register(vm: &mut VM) {
    vm.register_native("len",         native_len);
    vm.register_native("to_int",      native_to_int);
    vm.register_native("to_float",    native_to_float);
    vm.register_native("to_str",      native_to_str);
    vm.register_native("trim",        native_trim);
    vm.register_native("upper",       native_upper);
    vm.register_native("lower",       native_lower);
    vm.register_native("contains",    native_contains);
    vm.register_native("starts_with", native_starts_with);
    vm.register_native("ends_with",   native_ends_with);
    vm.register_native("replace",     native_replace);
    vm.register_native("split",       native_split);
    vm.register_native("append",      native_append);
    vm.register_native("pop",         native_pop);
    vm.register_native("keys",        native_keys);
}

// ── String functions ──────────────────────────────────────────────────────────

fn native_len(args: Vec<Value>) -> VmResult<Value> {
    expect_args("len", &args, 1)?;
    match &args[0] {
        Value::Str(s)   => Ok(Value::Int(s.chars().count() as i64)),
        Value::List(rc) => Ok(Value::Int(rc.borrow().len() as i64)),
        _ => Err(VmError::new("len() requires a str or list")),
    }
}

fn native_to_int(args: Vec<Value>) -> VmResult<Value> {
    expect_args("to_int", &args, 1)?;
    match &args[0] {
        Value::Int(v)   => Ok(Value::Int(*v)),
        Value::Float(v) => Ok(Value::Int(*v as i64)),
        Value::Str(s)   => s.trim().parse::<i64>()
            .map(Value::Int)
            .map_err(|_| VmError::new(format!("to_int(): cannot convert '{}' to int", s))),
        _ => Err(VmError::new("to_int() requires a str, int, or float")),
    }
}

fn native_to_float(args: Vec<Value>) -> VmResult<Value> {
    expect_args("to_float", &args, 1)?;
    match &args[0] {
        Value::Float(v) => Ok(Value::Float(*v)),
        Value::Int(v)   => Ok(Value::Float(*v as f64)),
        Value::Str(s)   => s.trim().parse::<f64>()
            .map(Value::Float)
            .map_err(|_| VmError::new(format!("to_float(): cannot convert '{}' to float", s))),
        _ => Err(VmError::new("to_float() requires a str, int, or float")),
    }
}

fn native_to_str(args: Vec<Value>) -> VmResult<Value> {
    expect_args("to_str", &args, 1)?;
    Ok(Value::Str(Rc::new(format!("{}", args[0]))))
}

fn native_trim(args: Vec<Value>) -> VmResult<Value> {
    expect_args("trim", &args, 1)?;
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(Rc::new(s.trim().to_string()))),
        _ => Err(VmError::new("trim() requires a str")),
    }
}

fn native_upper(args: Vec<Value>) -> VmResult<Value> {
    expect_args("upper", &args, 1)?;
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(Rc::new(s.to_uppercase()))),
        _ => Err(VmError::new("upper() requires a str")),
    }
}

fn native_lower(args: Vec<Value>) -> VmResult<Value> {
    expect_args("lower", &args, 1)?;
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(Rc::new(s.to_lowercase()))),
        _ => Err(VmError::new("lower() requires a str")),
    }
}

fn native_contains(args: Vec<Value>) -> VmResult<Value> {
    expect_args("contains", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
        _ => Err(VmError::new("contains() requires two str arguments")),
    }
}

fn native_starts_with(args: Vec<Value>) -> VmResult<Value> {
    expect_args("starts_with", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
        _ => Err(VmError::new("starts_with() requires two str arguments")),
    }
}

fn native_ends_with(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ends_with", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
        _ => Err(VmError::new("ends_with() requires two str arguments")),
    }
}

fn native_replace(args: Vec<Value>) -> VmResult<Value> {
    expect_args("replace", &args, 3)?;
    match (&args[0], &args[1], &args[2]) {
        (Value::Str(s), Value::Str(from), Value::Str(to)) => {
            Ok(Value::Str(Rc::new(s.replace(from.as_str(), to.as_str()))))
        }
        _ => Err(VmError::new("replace() requires three str arguments")),
    }
}

fn native_split(args: Vec<Value>) -> VmResult<Value> {
    expect_args("split", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(sep)) => {
            let parts: Vec<Value> = s.split(sep.as_str())
                .map(|p| Value::Str(Rc::new(p.to_string())))
                .collect();
            Ok(Value::List(Rc::new(RefCell::new(parts))))
        }
        _ => Err(VmError::new("split() requires two str arguments")),
    }
}

// ── List functions ────────────────────────────────────────────────────────────

/// append(list, value) → null
/// Adds a value to the end of the list in-place.
/// Because lists are Rc<RefCell<Vec<Value>>>, the Rc clone inside the VM
/// still points to the same RefCell — so borrow_mut() modifies the original.
fn native_append(args: Vec<Value>) -> VmResult<Value> {
    expect_args("append", &args, 2)?;
    match &args[0] {
        Value::List(rc) => {
            rc.borrow_mut().push(args[1].clone());
            Ok(Value::Null)
        }
        _ => Err(VmError::new("append() requires a list as first argument")),
    }
}

/// pop(list) → any
/// Removes and returns the last element of the list.
/// Returns null if the list is empty.
fn native_pop(args: Vec<Value>) -> VmResult<Value> {
    expect_args("pop", &args, 1)?;
    match &args[0] {
        Value::List(rc) => {
            let val = rc.borrow_mut().pop().unwrap_or(Value::Null);
            Ok(val)
        }
        _ => Err(VmError::new("pop() requires a list")),
    }
}

/// keys(list) → list
/// Returns a list of integer indices for the given list: [0, 1, 2, ..., n-1].
/// Useful for iterating with index when for-each is not yet available.
///
/// Example:
///   var items: list = ["a", "b", "c"]
///   var idx = keys(items)   # [0, 1, 2]
///   for i from 0 to len(idx):
///       print(i, items[i])
fn native_keys(args: Vec<Value>) -> VmResult<Value> {
    expect_args("keys", &args, 1)?;
    match &args[0] {
        Value::List(rc) => {
            let n = rc.borrow().len();
            let indices: Vec<Value> = (0..n).map(|i| Value::Int(i as i64)).collect();
            Ok(Value::List(Rc::new(RefCell::new(indices))))
        }
        _ => Err(VmError::new("keys() requires a list")),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}", name, expected, args.len()
        )))
    } else {
        Ok(())
    }
}