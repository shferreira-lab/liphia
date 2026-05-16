// stdlib/native/src/math.rs
//
// Math functions.
//
// Functions registered:
//   — Original —
//   sqrt(x)              → float
//   pow(base, exp)       → float
//   abs(x)               → int or float (matches input type)
//   floor(x)             → int
//   ceil(x)              → int
//   round(x)             → int
//   min(a, b)            → int or float
//   max(a, b)            → int or float
//   pi()                 → float    (3.141592653589793)
//   e()                  → float    (2.718281828459045)
//   log(x)               → float    natural log
//   log10(x)             → float
//   sin(x)               → float    radians
//   cos(x)               → float
//   tan(x)               → float
//
//   — Trigonometry (inverse) —
//   asin(x)              → float    arc sine,    x ∈ [-1, 1]
//   acos(x)              → float    arc cosine,  x ∈ [-1, 1]
//   atan(x)              → float    arc tangent
//   atan2(y, x)          → float    four-quadrant arc tangent
//
//   — Hyperbolic —
//   sinh(x)              → float
//   cosh(x)              → float
//   tanh(x)              → float
//
//   — Exponential / logarithm —
//   exp(x)               → float    e^x
//   log2(x)              → float    base-2 logarithm, x > 0
//   log_base(x, b)       → float    logarithm in arbitrary base b, x > 0, b > 0, b ≠ 1
//
//   — Number theory / combinatorics —
//   factorial(n)         → int      n! for n >= 0
//   gcd(a, b)            → int      greatest common divisor
//   lcm(a, b)            → int      least common multiple
//
//   — Geometry / vector —
//   hypot(a, b)          → float    sqrt(a² + b²)
//   deg_to_rad(x)        → float    degrees → radians
//   rad_to_deg(x)        → float    radians → degrees
//
//   — Utilities —
//   sign(x)              → int      -1, 0, or 1
//   clamp(x, lo, hi)     → int or float  (matches x type)
//   is_nan(x)            → bool
//   is_inf(x)            → bool

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

pub fn register(vm: &mut VM) {
    // ── Original ──────────────────────────────────────────────────────────────
    vm.register_native("sqrt",       native_sqrt);
    vm.register_native("pow",        native_pow);
    vm.register_native("abs",        native_abs);
    vm.register_native("floor",      native_floor);
    vm.register_native("ceil",       native_ceil);
    vm.register_native("round",      native_round);
    vm.register_native("min",        native_min);
    vm.register_native("max",        native_max);
    vm.register_native("pi",         native_pi);
    vm.register_native("e",          native_e);
    vm.register_native("log",        native_log);
    vm.register_native("log10",      native_log10);
    vm.register_native("sin",        native_sin);
    vm.register_native("cos",        native_cos);
    vm.register_native("tan",        native_tan);

    // ── Trigonometry (inverse) ────────────────────────────────────────────────
    vm.register_native("asin",       native_asin);
    vm.register_native("acos",       native_acos);
    vm.register_native("atan",       native_atan);
    vm.register_native("atan2",      native_atan2);

    // ── Hyperbolic ────────────────────────────────────────────────────────────
    vm.register_native("sinh",       native_sinh);
    vm.register_native("cosh",       native_cosh);
    vm.register_native("tanh",       native_tanh);

    // ── Exponential / logarithm ───────────────────────────────────────────────
    vm.register_native("exp",        native_exp);
    vm.register_native("log2",       native_log2);
    vm.register_native("log_base",   native_log_base);

    // ── Number theory / combinatorics ─────────────────────────────────────────
    vm.register_native("factorial",  native_factorial);
    vm.register_native("gcd",        native_gcd);
    vm.register_native("lcm",        native_lcm);

    // ── Geometry / vector ─────────────────────────────────────────────────────
    vm.register_native("hypot",      native_hypot);
    vm.register_native("deg_to_rad", native_deg_to_rad);
    vm.register_native("rad_to_deg", native_rad_to_deg);

    // ── Utilities ─────────────────────────────────────────────────────────────
    vm.register_native("sign",       native_sign);
    vm.register_native("clamp",      native_clamp);
    vm.register_native("is_nan",     native_is_nan);
    vm.register_native("is_inf",     native_is_inf);
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn to_f64(v: &Value, ctx: &str) -> VmResult<f64> {
    match v {
        Value::Int(i)   => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(VmError::new(format!("{}: argument must be int or float", ctx))),
    }
}

fn to_i64(v: &Value, ctx: &str) -> VmResult<i64> {
    match v {
        Value::Int(i) => Ok(*i),
        _ => Err(VmError::new(format!("{}: argument must be int", ctx))),
    }
}

fn expect_args(name: &str, args: &[Value], expected: usize) -> VmResult<()> {
    if args.len() != expected {
        Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}", name, expected, args.len()
        )))
    } else { Ok(()) }
}

// ── Original functions ────────────────────────────────────────────────────────

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

// ── Trigonometry (inverse) ────────────────────────────────────────────────────

fn native_asin(args: Vec<Value>) -> VmResult<Value> {
    expect_args("asin", &args, 1)?;
    let x = to_f64(&args[0], "asin")?;
    if x < -1.0 || x > 1.0 {
        return Err(VmError::new("asin(): argument must be in [-1, 1]"));
    }
    Ok(Value::Float(x.asin()))
}

fn native_acos(args: Vec<Value>) -> VmResult<Value> {
    expect_args("acos", &args, 1)?;
    let x = to_f64(&args[0], "acos")?;
    if x < -1.0 || x > 1.0 {
        return Err(VmError::new("acos(): argument must be in [-1, 1]"));
    }
    Ok(Value::Float(x.acos()))
}

fn native_atan(args: Vec<Value>) -> VmResult<Value> {
    expect_args("atan", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "atan")?.atan()))
}

fn native_atan2(args: Vec<Value>) -> VmResult<Value> {
    expect_args("atan2", &args, 2)?;
    let y = to_f64(&args[0], "atan2")?;
    let x = to_f64(&args[1], "atan2")?;
    Ok(Value::Float(y.atan2(x)))
}

// ── Hyperbolic ────────────────────────────────────────────────────────────────

fn native_sinh(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sinh", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "sinh")?.sinh()))
}

fn native_cosh(args: Vec<Value>) -> VmResult<Value> {
    expect_args("cosh", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "cosh")?.cosh()))
}

fn native_tanh(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tanh", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "tanh")?.tanh()))
}

// ── Exponential / logarithm ───────────────────────────────────────────────────

fn native_exp(args: Vec<Value>) -> VmResult<Value> {
    expect_args("exp", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "exp")?.exp()))
}

fn native_log2(args: Vec<Value>) -> VmResult<Value> {
    expect_args("log2", &args, 1)?;
    let x = to_f64(&args[0], "log2")?;
    if x <= 0.0 { return Err(VmError::new("log2(): argument must be > 0")); }
    Ok(Value::Float(x.log2()))
}

fn native_log_base(args: Vec<Value>) -> VmResult<Value> {
    expect_args("log_base", &args, 2)?;
    let x = to_f64(&args[0], "log_base")?;
    let b = to_f64(&args[1], "log_base")?;
    if x <= 0.0 { return Err(VmError::new("log_base(): x must be > 0")); }
    if b <= 0.0 || (b - 1.0).abs() < f64::EPSILON {
        return Err(VmError::new("log_base(): base must be > 0 and != 1"));
    }
    Ok(Value::Float(x.ln() / b.ln()))
}

// ── Number theory / combinatorics ─────────────────────────────────────────────

fn native_factorial(args: Vec<Value>) -> VmResult<Value> {
    expect_args("factorial", &args, 1)?;
    let n = to_i64(&args[0], "factorial")?;
    if n < 0 {
        return Err(VmError::new("factorial(): argument must be >= 0"));
    }
    if n > 20 {
        // 21! overflows i64 (max is 9_223_372_036_854_775_807, 20! = 2_432_902_008_176_640_000)
        return Err(VmError::new("factorial(): argument must be <= 20 to avoid i64 overflow"));
    }
    let result: i64 = (1..=n).product();
    Ok(Value::Int(result))
}

fn gcd_inner(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn native_gcd(args: Vec<Value>) -> VmResult<Value> {
    expect_args("gcd", &args, 2)?;
    let a = to_i64(&args[0], "gcd")?;
    let b = to_i64(&args[1], "gcd")?;
    Ok(Value::Int(gcd_inner(a, b)))
}

fn native_lcm(args: Vec<Value>) -> VmResult<Value> {
    expect_args("lcm", &args, 2)?;
    let a = to_i64(&args[0], "lcm")?;
    let b = to_i64(&args[1], "lcm")?;
    if a == 0 || b == 0 { return Ok(Value::Int(0)); }
    Ok(Value::Int((a / gcd_inner(a, b)) * b))
}

// ── Geometry / vector ─────────────────────────────────────────────────────────

fn native_hypot(args: Vec<Value>) -> VmResult<Value> {
    expect_args("hypot", &args, 2)?;
    let a = to_f64(&args[0], "hypot")?;
    let b = to_f64(&args[1], "hypot")?;
    Ok(Value::Float(a.hypot(b)))
}

fn native_deg_to_rad(args: Vec<Value>) -> VmResult<Value> {
    expect_args("deg_to_rad", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "deg_to_rad")?.to_radians()))
}

fn native_rad_to_deg(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rad_to_deg", &args, 1)?;
    Ok(Value::Float(to_f64(&args[0], "rad_to_deg")?.to_degrees()))
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn native_sign(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sign", &args, 1)?;
    match &args[0] {
        Value::Int(i)   => Ok(Value::Int(i.signum())),
        Value::Float(f) => Ok(Value::Int(if *f > 0.0 { 1 } else if *f < 0.0 { -1 } else { 0 })),
        _ => Err(VmError::new("sign(): argument must be int or float")),
    }
}

fn native_clamp(args: Vec<Value>) -> VmResult<Value> {
    expect_args("clamp", &args, 3)?;
    match (&args[0], &args[1], &args[2]) {
        (Value::Int(x),   Value::Int(lo),   Value::Int(hi))   => Ok(Value::Int((*x).clamp(*lo, *hi))),
        (Value::Float(x), Value::Float(lo), Value::Float(hi)) => Ok(Value::Float(x.clamp(*lo, *hi))),
        // mixed: promote to float
        _ => {
            let x  = to_f64(&args[0], "clamp")?;
            let lo = to_f64(&args[1], "clamp")?;
            let hi = to_f64(&args[2], "clamp")?;
            Ok(Value::Float(x.clamp(lo, hi)))
        }
    }
}

fn native_is_nan(args: Vec<Value>) -> VmResult<Value> {
    expect_args("is_nan", &args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_nan())),
        Value::Int(_)   => Ok(Value::Bool(false)),
        _ => Err(VmError::new("is_nan(): argument must be int or float")),
    }
}

fn native_is_inf(args: Vec<Value>) -> VmResult<Value> {
    expect_args("is_inf", &args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_infinite())),
        Value::Int(_)   => Ok(Value::Bool(false)),
        _ => Err(VmError::new("is_inf(): argument must be int or float")),
    }
}
