// stdlib/native/src/stats.rs
//
// Statistical functions operating on Liphia lists.
//
// Functions registered:
//   sum(list)         → float   sum of all elements
//   mean(list)        → float   arithmetic mean
//   min_list(list)    → float   minimum value
//   max_list(list)    → float   maximum value
//   median(list)      → float   median value
//   variance(list)    → float   population variance
//   stdev(list)       → float   population standard deviation
//   count(list)       → int     number of elements (same as len)

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

pub fn register(vm: &mut VM) {
    vm.register_native("sum",      native_sum);
    vm.register_native("mean",     native_mean);
    vm.register_native("min_list", native_min_list);
    vm.register_native("max_list", native_max_list);
    vm.register_native("median",   native_median);
    vm.register_native("variance", native_variance);
    vm.register_native("stdev",    native_stdev);
    vm.register_native("count",    native_count);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_floats(args: &[Value], fn_name: &str) -> VmResult<Vec<f64>> {
    if args.len() != 1 {
        return Err(VmError::new(format!("{}() expects 1 argument (list)", fn_name)));
    }
    match &args[0] {
        Value::List(rc) => {
            let items = rc.borrow();
            if items.is_empty() {
                return Err(VmError::new(format!("{}(): list must not be empty", fn_name)));
            }
            items.iter().map(|v| match v {
                Value::Int(i)   => Ok(*i as f64),
                Value::Float(f) => Ok(*f),
                _ => Err(VmError::new(format!(
                    "{}(): list must contain only int or float values", fn_name
                ))),
            }).collect()
        }
        _ => Err(VmError::new(format!("{}(): argument must be a list", fn_name))),
    }
}

// ── Functions ─────────────────────────────────────────────────────────────────

fn native_sum(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "sum")?;
    Ok(Value::Float(v.iter().sum()))
}

fn native_mean(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "mean")?;
    Ok(Value::Float(v.iter().sum::<f64>() / v.len() as f64))
}

fn native_min_list(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "min_list")?;
    Ok(Value::Float(v.iter().cloned().fold(f64::INFINITY, f64::min)))
}

fn native_max_list(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "max_list")?;
    Ok(Value::Float(v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
}

fn native_median(args: Vec<Value>) -> VmResult<Value> {
    let mut v = extract_floats(&args, "median")?;
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    let result = if v.len() % 2 == 0 {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    };
    Ok(Value::Float(result))
}

fn native_variance(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "variance")?;
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var  = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
    Ok(Value::Float(var))
}

fn native_stdev(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_floats(&args, "stdev")?;
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var  = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
    Ok(Value::Float(var.sqrt()))
}

fn native_count(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("count() expects 1 argument (list)"));
    }
    match &args[0] {
        Value::List(rc) => Ok(Value::Int(rc.borrow().len() as i64)),
        _ => Err(VmError::new("count(): argument must be a list")),
    }
}
