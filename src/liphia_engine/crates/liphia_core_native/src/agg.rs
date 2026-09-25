// liphia_core_native/src/agg.rs
//
// Basic aggregation over lists of numbers. Descriptive statistics beyond
// these four (median, variance, percentiles...) live in the `num` package.
//
// Functions registered:
//   sum(list)        -> int | float   0 for an empty list
//   mean(list)       -> float         error for an empty list
//   min_list(list)   -> int | float   error for an empty list
//   max_list(list)   -> int | float   error for an empty list
//
// The list must be homogeneous: all int or all float. A mixed list is an
// error, consistent with the rule that int and float never mix implicitly.
// Results keep the element type, except mean, which is always float.
// An int sum that does not fit in i64 is an error.

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::util::expect_args;

pub fn register(vm: &mut VM) {
    vm.register_native("sum", native_sum);
    vm.register_native("mean", native_mean);
    vm.register_native("min_list", native_min_list);
    vm.register_native("max_list", native_max_list);
}

// ── Homogeneous numeric list ──────────────────────────────────────────────────

enum Numbers {
    Ints(Vec<i64>),
    Floats(Vec<f64>),
}

impl Numbers {
    fn len(&self) -> usize {
        match self {
            Numbers::Ints(v) => v.len(),
            Numbers::Floats(v) => v.len(),
        }
    }
}

// Reads the single list argument. The element type is fixed by the first
// element; every other element must match it. An empty list is returned
// as an empty Ints so that sum([]) is the int 0.
fn numbers(name: &str, args: &[Value]) -> VmResult<Numbers> {
    expect_args(name, args, 1)?;
    let items = match &args[0] {
        Value::List(rc) => rc.borrow(),
        _ => return Err(VmError::new(format!("{}(): argument must be a list", name))),
    };
    let mixed = || {
        VmError::new(format!(
            "{}(): list must be all int or all float (no mixing)",
            name
        ))
    };
    match items.first() {
        None => Ok(Numbers::Ints(vec![])),
        Some(Value::Int(_)) => items
            .iter()
            .map(|v| match v {
                Value::Int(i) => Ok(*i),
                _ => Err(mixed()),
            })
            .collect::<VmResult<Vec<i64>>>()
            .map(Numbers::Ints),
        Some(Value::Float(_)) => items
            .iter()
            .map(|v| match v {
                Value::Float(f) => Ok(*f),
                _ => Err(mixed()),
            })
            .collect::<VmResult<Vec<f64>>>()
            .map(Numbers::Floats),
        Some(_) => Err(VmError::new(format!(
            "{}(): list must contain only int or float",
            name
        ))),
    }
}

fn non_empty(name: &str, values: &Numbers) -> VmResult<()> {
    if values.len() == 0 {
        return Err(VmError::new(format!("{}(): list must not be empty", name)));
    }
    Ok(())
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_sum(args: Vec<Value>) -> VmResult<Value> {
    match numbers("sum", &args)? {
        Numbers::Ints(v) => v
            .iter()
            .try_fold(0i64, |acc, x| acc.checked_add(*x))
            .map(Value::Int)
            .ok_or_else(|| VmError::new("sum(): integer overflow")),
        Numbers::Floats(v) => Ok(Value::Float(v.iter().sum())),
    }
}

// Int elements are summed in i128 before the division, so the mean of
// large ints never overflows on the way.
fn native_mean(args: Vec<Value>) -> VmResult<Value> {
    let values = numbers("mean", &args)?;
    non_empty("mean", &values)?;
    let mean = match &values {
        Numbers::Ints(v) => v.iter().map(|x| *x as i128).sum::<i128>() as f64 / v.len() as f64,
        Numbers::Floats(v) => v.iter().sum::<f64>() / v.len() as f64,
    };
    Ok(Value::Float(mean))
}

fn native_min_list(args: Vec<Value>) -> VmResult<Value> {
    let values = numbers("min_list", &args)?;
    non_empty("min_list", &values)?;
    Ok(match values {
        Numbers::Ints(v) => Value::Int(*v.iter().min().unwrap()),
        Numbers::Floats(v) => Value::Float(v.into_iter().fold(f64::INFINITY, f64::min)),
    })
}

fn native_max_list(args: Vec<Value>) -> VmResult<Value> {
    let values = numbers("max_list", &args)?;
    non_empty("max_list", &values)?;
    Ok(match values {
        Numbers::Ints(v) => Value::Int(*v.iter().max().unwrap()),
        Numbers::Floats(v) => Value::Float(v.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    })
}
