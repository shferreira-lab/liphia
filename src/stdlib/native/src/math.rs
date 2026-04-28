// stdlib/native/src/math.rs
//
// Math functions.
//
// Functions registered:
//   sqrt(x)         → float
//   pow(base, exp)  → float
//   abs(x)          → int or float (matches input type)
//   floor(x)        → int
//   ceil(x)         → int
//   round(x)        → int
//   min(a, b)       → int or float
//   max(a, b)       → int or float
//   pi()            → float    (3.141592653589793)
//   e()             → float    (2.718281828459045)
//   log(x)          → float    natural log
//   log10(x)        → float
//   sin(x)          → float    radians
//   cos(x)          → float
//   tan(x)          → float

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

pub fn register(vm: &mut VM) {
    vm.register_native("sqrt",  native_sqrt);
    vm.register_native("pow",   native_pow);
    vm.register_native("abs",   native_abs);
    vm.register_native("floor", native_floor);
    vm.register_native("ceil",  native_ceil);
    vm.register_native("round", native_round);
    vm.register_native("min",   native_min);
    vm.register_native("max",   native_max);
    vm.register_native("pi",    native_pi);
    vm.register_native("e",     native_e);
    vm.register_native("log",   native_log);
    vm.register_native("log10", native_log10);
    vm.register_native("sin",   native_sin);
    vm.register_native("cos",   native_cos);
    vm.register_native("tan",   native_tan);
}

fn to_f64(v: &Value, ctx: &str) -> VmResult<f64> {
    match v {
        Value::Int(i)   => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(VmError::new(format!("{}: argument must be int or float", ctx))),
    }
}

fn native_sqrt(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sqrt", &args, 1)?;
    let x = to_f64(&args[0], "sqrt")?;
    if x < 0.0 { return Err(VmError::new("sqrt(): argument must be >= 0")); }
    Ok(Value::Float(x.sqrt()))
}

fn native_pow(args: Vec<Value>) -> VmResult<Value> {
    expect_args("pow", &args, 2)?;
    let base = to_f64(&args[0], "pow")?;
    let exp  = to_f64(&args[1], "pow")?;
    Ok(Value::Float(base.powf(exp)))
}

fn native_abs(args: Vec<Value>) -> VmResult<Value> {
    expect_args("abs", &args, 1)?;
    match &args[0] {
        Value::Int(i)   => Ok(Value::Int(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(VmError::new("abs(): argument must be int or float")),
    }
}

fn native_floor(args: Vec<Value>) -> VmResult<Value> {
    expect_args("floor", &args, 1)?;
    Ok(Value::Int(to_f64(&args[0], "floor")?.floor() as i64))
}

fn native_ceil(args: Vec<Value>) -> VmResult<Value> {
    expect_args("ceil", &args, 1)?;
    Ok(Value::Int(to_f64(&args[0], "ceil")?.ceil() as i64))
}

fn native_round(args: Vec<Value>) -> VmResult<Value> {
    expect_args("round", &args, 1)?;
    Ok(Value::Int(to_f64(&args[0], "round")?.round() as i64))
}

fn native_min(args: Vec<Value>) -> VmResult<Value> {
    expect_args("min", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a),   Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a.min(*b as f64))),
        _ => Err(VmError::new("min(): arguments must be int or float")),
    }
}

fn native_max(args: Vec<Value>) -> VmResult<Value> {
    expect_args("max", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a),   Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a.max(*b as f64))),
        _ => Err(VmError::new("max(): arguments must be int or float")),
    }
}

fn native_pi(args: Vec<Value>) -> VmResult<Value> {
    expect_args("pi", &args, 0)?;
    Ok(Value::Float(std::f64::consts::PI))
}

fn native_e(args: Vec<Value>) -> VmResult<Value> {
    expect_args("e", &args, 0)?;
    Ok(Value::Float(std::f64::consts::E))
}

fn native_log(args: Vec<Value>) -> VmResult<Value> {
    expect_args("log", &args, 1)?;
    let x = to_f64(&args[0], "log")?;
    if x <= 0.0 { return Err(VmError::new("log(): argument must be > 0")); }
    Ok(Value::Float(x.ln()))
}

fn native_log10(args: Vec<Value>) -> VmResult<Value> {
    expect_args("log10", &args, 1)?;
    let x = to_f64(&args[0], "log10")?;
    if x <= 0.0 { return Err(VmError::new("log10(): argument must be > 0")); }
    Ok(Value::Float(x.log10()))
}

fn native_sin(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sin", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "sin")?.sin()))
}

fn native_cos(args: Vec<Value>) -> VmResult<Value> {
    expect_args("cos", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "cos")?.cos()))
}

fn native_tan(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tan", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "tan")?.tan()))
}

fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}", name, expected, args.len()
        )))
    } else { Ok(()) }
}
