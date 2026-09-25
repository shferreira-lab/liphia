// liphia_core_native/src/math.rs
//
// Scalar math. Every function takes and returns single numbers; list and
// matrix math lives in the `num` package.
//
// Functions registered:
//   sqrt(x), pow(base, exp), exp(x)                       -> float
//   log(x), log10(x), log2(x), log_base(x, b)             -> float
//   sin, cos, tan, asin, acos, atan (x), atan2(y, x)      -> float
//   sinh, cosh, tanh (x)                                  -> float
//   hypot(a, b), deg_to_rad(x), rad_to_deg(x)             -> float
//   pi(), e(), inf(), nan()                               -> float
//   abs(x)                                                -> same type as x
//   floor(x), ceil(x), round(x)                           -> int
//   min(a, b), max(a, b)                                  -> same type as args
//   clamp(x, lo, hi)                                      -> same type as args
//   sign(x)                                               -> int
//   factorial(n), gcd(a, b), lcm(a, b)                    -> int
//   is_nan(x), is_inf(x)                                  -> bool
//
// Type rules, aligned with the operator semantics:
//   - min, max and clamp never mix int and float: all arguments must share
//     one type, otherwise it is an error (no implicit promotion).
//   - Integer results that do not fit in i64 are errors, never wrapped
//     (abs(MIN), lcm overflow, floor/ceil/round of huge floats).
//   - floor, ceil and round of NaN or infinity are errors.

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::util::{expect_args, int_arg, num_arg};

pub fn register(vm: &mut VM) {
    vm.register_native("sqrt", native_sqrt);
    vm.register_native("pow", native_pow);
    vm.register_native("exp", native_exp);
    vm.register_native("log", native_log);
    vm.register_native("log10", native_log10);
    vm.register_native("log2", native_log2);
    vm.register_native("log_base", native_log_base);

    vm.register_native("sin", native_sin);
    vm.register_native("cos", native_cos);
    vm.register_native("tan", native_tan);
    vm.register_native("asin", native_asin);
    vm.register_native("acos", native_acos);
    vm.register_native("atan", native_atan);
    vm.register_native("atan2", native_atan2);
    vm.register_native("sinh", native_sinh);
    vm.register_native("cosh", native_cosh);
    vm.register_native("tanh", native_tanh);

    vm.register_native("hypot", native_hypot);
    vm.register_native("deg_to_rad", native_deg_to_rad);
    vm.register_native("rad_to_deg", native_rad_to_deg);

    vm.register_native("pi", native_pi);
    vm.register_native("e", native_e);
    vm.register_native("inf", native_inf);
    vm.register_native("nan", native_nan);

    vm.register_native("abs", native_abs);
    vm.register_native("floor", native_floor);
    vm.register_native("ceil", native_ceil);
    vm.register_native("round", native_round);
    vm.register_native("min", native_min);
    vm.register_native("max", native_max);
    vm.register_native("clamp", native_clamp);
    vm.register_native("sign", native_sign);

    vm.register_native("factorial", native_factorial);
    vm.register_native("gcd", native_gcd);
    vm.register_native("lcm", native_lcm);

    vm.register_native("is_nan", native_is_nan);
    vm.register_native("is_inf", native_is_inf);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// Applies a float -> float function to the single numeric argument.
fn unary(name: &str, args: &[Value], f: fn(f64) -> f64) -> VmResult<Value> {
    expect_args(name, args, 1)?;
    Ok(Value::Float(f(num_arg(&args[0], name)?)))
}

fn positive_arg(name: &str, args: &[Value]) -> VmResult<f64> {
    expect_args(name, args, 1)?;
    let x = num_arg(&args[0], name)?;
    if x <= 0.0 {
        return Err(VmError::new(format!("{}(): argument must be > 0", name)));
    }
    Ok(x)
}

// Converts a float to i64 for floor/ceil/round, rejecting NaN, infinity
// and values outside the i64 range instead of saturating silently.
fn float_to_int(name: &str, x: f64) -> VmResult<Value> {
    if !x.is_finite() {
        return Err(VmError::new(format!("{}(): argument must be finite", name)));
    }
    if x < i64::MIN as f64 || x >= i64::MAX as f64 {
        return Err(VmError::new(format!("{}(): result does not fit in int", name)));
    }
    Ok(Value::Int(x as i64))
}

fn rounding(name: &str, args: &[Value], f: fn(f64) -> f64) -> VmResult<Value> {
    expect_args(name, args, 1)?;
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(x) => float_to_int(name, f(*x)),
        _ => Err(VmError::new(format!("{}(): argument must be int or float", name))),
    }
}

fn same_type_error(name: &str) -> VmError {
    VmError::new(format!(
        "{}(): arguments must all be int or all be float (no mixing)",
        name
    ))
}

// ── Powers, exponentials, logarithms ─────────────────────────────────────────

fn native_sqrt(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sqrt", &args, 1)?;
    let x = num_arg(&args[0], "sqrt")?;
    if x < 0.0 {
        return Err(VmError::new("sqrt(): argument must be >= 0"));
    }
    Ok(Value::Float(x.sqrt()))
}

fn native_pow(args: Vec<Value>) -> VmResult<Value> {
    expect_args("pow", &args, 2)?;
    let base = num_arg(&args[0], "pow")?;
    let exp = num_arg(&args[1], "pow")?;
    Ok(Value::Float(base.powf(exp)))
}

fn native_exp(args: Vec<Value>) -> VmResult<Value> {
    unary("exp", &args, f64::exp)
}

fn native_log(args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Float(positive_arg("log", &args)?.ln()))
}

fn native_log10(args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Float(positive_arg("log10", &args)?.log10()))
}

fn native_log2(args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Float(positive_arg("log2", &args)?.log2()))
}

fn native_log_base(args: Vec<Value>) -> VmResult<Value> {
    expect_args("log_base", &args, 2)?;
    let x = num_arg(&args[0], "log_base")?;
    let b = num_arg(&args[1], "log_base")?;
    if x <= 0.0 {
        return Err(VmError::new("log_base(): x must be > 0"));
    }
    if b <= 0.0 || b == 1.0 {
        return Err(VmError::new("log_base(): base must be > 0 and != 1"));
    }
    Ok(Value::Float(x.ln() / b.ln()))
}

// ── Trigonometry ──────────────────────────────────────────────────────────────

fn native_sin(args: Vec<Value>) -> VmResult<Value> {
    unary("sin", &args, f64::sin)
}

fn native_cos(args: Vec<Value>) -> VmResult<Value> {
    unary("cos", &args, f64::cos)
}

fn native_tan(args: Vec<Value>) -> VmResult<Value> {
    unary("tan", &args, f64::tan)
}

fn native_asin(args: Vec<Value>) -> VmResult<Value> {
    expect_args("asin", &args, 1)?;
    let x = num_arg(&args[0], "asin")?;
    if !(-1.0..=1.0).contains(&x) {
        return Err(VmError::new("asin(): argument must be in [-1, 1]"));
    }
    Ok(Value::Float(x.asin()))
}

fn native_acos(args: Vec<Value>) -> VmResult<Value> {
    expect_args("acos", &args, 1)?;
    let x = num_arg(&args[0], "acos")?;
    if !(-1.0..=1.0).contains(&x) {
        return Err(VmError::new("acos(): argument must be in [-1, 1]"));
    }
    Ok(Value::Float(x.acos()))
}

fn native_atan(args: Vec<Value>) -> VmResult<Value> {
    unary("atan", &args, f64::atan)
}

fn native_atan2(args: Vec<Value>) -> VmResult<Value> {
    expect_args("atan2", &args, 2)?;
    let y = num_arg(&args[0], "atan2")?;
    let x = num_arg(&args[1], "atan2")?;
    Ok(Value::Float(y.atan2(x)))
}

fn native_sinh(args: Vec<Value>) -> VmResult<Value> {
    unary("sinh", &args, f64::sinh)
}

fn native_cosh(args: Vec<Value>) -> VmResult<Value> {
    unary("cosh", &args, f64::cosh)
}

fn native_tanh(args: Vec<Value>) -> VmResult<Value> {
    unary("tanh", &args, f64::tanh)
}

// ── Geometry ──────────────────────────────────────────────────────────────────

fn native_hypot(args: Vec<Value>) -> VmResult<Value> {
    expect_args("hypot", &args, 2)?;
    let a = num_arg(&args[0], "hypot")?;
    let b = num_arg(&args[1], "hypot")?;
    Ok(Value::Float(a.hypot(b)))
}

fn native_deg_to_rad(args: Vec<Value>) -> VmResult<Value> {
    unary("deg_to_rad", &args, f64::to_radians)
}

fn native_rad_to_deg(args: Vec<Value>) -> VmResult<Value> {
    unary("rad_to_deg", &args, f64::to_degrees)
}

// ── Constants ─────────────────────────────────────────────────────────────────

fn native_pi(args: Vec<Value>) -> VmResult<Value> {
    expect_args("pi", &args, 0)?;
    Ok(Value::Float(std::f64::consts::PI))
}

fn native_e(args: Vec<Value>) -> VmResult<Value> {
    expect_args("e", &args, 0)?;
    Ok(Value::Float(std::f64::consts::E))
}

// Float division by zero is an error, so infinity and NaN are only
// reachable explicitly through these two constants.
fn native_inf(args: Vec<Value>) -> VmResult<Value> {
    expect_args("inf", &args, 0)?;
    Ok(Value::Float(f64::INFINITY))
}

fn native_nan(args: Vec<Value>) -> VmResult<Value> {
    expect_args("nan", &args, 0)?;
    Ok(Value::Float(f64::NAN))
}

// ── Rounding, sign, comparison ────────────────────────────────────────────────

fn native_abs(args: Vec<Value>) -> VmResult<Value> {
    expect_args("abs", &args, 1)?;
    match &args[0] {
        Value::Int(i) => i
            .checked_abs()
            .map(Value::Int)
            .ok_or_else(|| VmError::new("abs(): integer overflow")),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(VmError::new("abs(): argument must be int or float")),
    }
}

fn native_floor(args: Vec<Value>) -> VmResult<Value> {
    rounding("floor", &args, f64::floor)
}

fn native_ceil(args: Vec<Value>) -> VmResult<Value> {
    rounding("ceil", &args, f64::ceil)
}

fn native_round(args: Vec<Value>) -> VmResult<Value> {
    rounding("round", &args, f64::round)
}

fn native_min(args: Vec<Value>) -> VmResult<Value> {
    expect_args("min", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        _ => Err(same_type_error("min")),
    }
}

fn native_max(args: Vec<Value>) -> VmResult<Value> {
    expect_args("max", &args, 2)?;
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        _ => Err(same_type_error("max")),
    }
}

// Rust's clamp panics when lo > hi; that case is checked first so it
// surfaces as a Liphia error instead of aborting the VM.
fn native_clamp(args: Vec<Value>) -> VmResult<Value> {
    expect_args("clamp", &args, 3)?;
    match (&args[0], &args[1], &args[2]) {
        (Value::Int(x), Value::Int(lo), Value::Int(hi)) => {
            if lo > hi {
                return Err(VmError::new("clamp(): lo must be <= hi"));
            }
            Ok(Value::Int((*x).clamp(*lo, *hi)))
        }
        (Value::Float(x), Value::Float(lo), Value::Float(hi)) => {
            if lo.is_nan() || hi.is_nan() || lo > hi {
                return Err(VmError::new("clamp(): lo must be <= hi"));
            }
            Ok(Value::Float(x.clamp(*lo, *hi)))
        }
        _ => Err(same_type_error("clamp")),
    }
}

fn native_sign(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sign", &args, 1)?;
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(i.signum())),
        Value::Float(f) => {
            if f.is_nan() {
                return Err(VmError::new("sign(): argument is NaN"));
            }
            Ok(Value::Int(if *f > 0.0 {
                1
            } else if *f < 0.0 {
                -1
            } else {
                0
            }))
        }
        _ => Err(VmError::new("sign(): argument must be int or float")),
    }
}

// ── Integer math ──────────────────────────────────────────────────────────────

fn native_factorial(args: Vec<Value>) -> VmResult<Value> {
    expect_args("factorial", &args, 1)?;
    let n = int_arg(&args[0], "factorial")?;
    if n < 0 {
        return Err(VmError::new("factorial(): argument must be >= 0"));
    }
    // 20! is the largest factorial that fits in i64.
    if n > 20 {
        return Err(VmError::new("factorial(): argument must be <= 20 (int overflow)"));
    }
    Ok(Value::Int((1..=n).product()))
}

// gcd on u64 so that i64::MIN has a representable absolute value; the
// result only fails when it does not fit back into i64.
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn native_gcd(args: Vec<Value>) -> VmResult<Value> {
    expect_args("gcd", &args, 2)?;
    let a = int_arg(&args[0], "gcd")?;
    let b = int_arg(&args[1], "gcd")?;
    let g = gcd_u64(a.unsigned_abs(), b.unsigned_abs());
    i64::try_from(g)
        .map(Value::Int)
        .map_err(|_| VmError::new("gcd(): integer overflow"))
}

fn native_lcm(args: Vec<Value>) -> VmResult<Value> {
    expect_args("lcm", &args, 2)?;
    let a = int_arg(&args[0], "lcm")?;
    let b = int_arg(&args[1], "lcm")?;
    if a == 0 || b == 0 {
        return Ok(Value::Int(0));
    }
    let (ua, ub) = (a.unsigned_abs(), b.unsigned_abs());
    (ua / gcd_u64(ua, ub))
        .checked_mul(ub)
        .and_then(|l| i64::try_from(l).ok())
        .map(Value::Int)
        .ok_or_else(|| VmError::new("lcm(): integer overflow"))
}

// ── Float predicates ──────────────────────────────────────────────────────────

fn native_is_nan(args: Vec<Value>) -> VmResult<Value> {
    expect_args("is_nan", &args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_nan())),
        Value::Int(_) => Ok(Value::Bool(false)),
        _ => Err(VmError::new("is_nan(): argument must be int or float")),
    }
}

fn native_is_inf(args: Vec<Value>) -> VmResult<Value> {
    expect_args("is_inf", &args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_infinite())),
        Value::Int(_) => Ok(Value::Bool(false)),
        _ => Err(VmError::new("is_inf(): argument must be int or float")),
    }
}
