// packages/num/native/src/vector.rs
//
// Vectors and matrices over plain Liphia lists of numbers.
//
// Functions registered:
//   dot(a, b) -> float                    norm(v) -> float
//   vec_add(a, b), vec_sub(a, b), vec_mul(a, b) -> list   element-wise
//   vec_scale(v, k) -> list               argmax(v) -> int
//   matrix_new(rows, cols, fill) -> list  row-major flat list
//   matrix_mul(a, b, rows, inner, cols) -> list
//   matrix_add(a, b) -> list              transpose(m, rows, cols) -> list
//   normalize(v) -> list      min-max scaling to [0, 1]
//   standardize(v) -> list    population z-scores (divides by n)
//   clip(v, min, max) -> list
//   linspace(start, end, n) -> list       arange(start, end, step) -> list
//   cosine_similarity(a, b), euclidean_dist(a, b), manhattan_dist(a, b) -> float
//
// Matrices are flat row-major lists; their shape travels as explicit
// rows/cols arguments. A real ndarray value is planned for a later version.

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

pub fn register(vm: &mut VM) {
    vm.register_native("dot",                    native_dot);
    vm.register_native("norm",                   native_norm);
    vm.register_native("vec_add",                native_vec_add);
    vm.register_native("vec_sub",                native_vec_sub);
    vm.register_native("vec_mul",                native_vec_mul);
    vm.register_native("vec_scale",              native_vec_scale);
    vm.register_native("argmax",                 native_argmax);
    vm.register_native("matrix_new",             native_matrix_new);
    vm.register_native("matrix_mul",             native_matrix_mul);
    vm.register_native("matrix_add",             native_matrix_add);
    vm.register_native("transpose",              native_transpose);
    vm.register_native("normalize",              native_normalize);
    vm.register_native("standardize",            native_standardize);
    vm.register_native("clip",                   native_clip);
    vm.register_native("linspace",               native_linspace);
    vm.register_native("arange",                 native_arange);
    vm.register_native("cosine_similarity",      native_cosine_similarity);
    vm.register_native("euclidean_dist",         native_euclidean_dist);
    vm.register_native("manhattan_dist",         native_manhattan_dist);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn expect_args(name: &str, args: &[Value], n: usize) -> VmResult<()> {
    if args.len() != n {
        Err(VmError::new(format!(
            "{}() expects {} argument(s), got {}", name, n, args.len()
        )))
    } else {
        Ok(())
    }
}

fn to_f64(v: &Value, ctx: &str) -> VmResult<f64> {
    match v {
        Value::Int(i)   => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(VmError::new(format!("{}: argument must be int or float", ctx))),
    }
}

fn list_to_f64s(v: &Value, ctx: &str) -> VmResult<Vec<f64>> {
    match v {
        Value::List(rc) => rc.borrow().iter().map(|x| to_f64(x, ctx)).collect(),
        _ => Err(VmError::new(format!("{}: argument must be a list", ctx))),
    }
}

fn f64s_to_list(v: Vec<f64>) -> Value {
    Value::List(Rc::new(RefCell::new(
        v.into_iter().map(Value::Float).collect(),
    )))
}

fn same_len(a: &[f64], b: &[f64], ctx: &str) -> VmResult<()> {
    if a.len() != b.len() {
        Err(VmError::new(format!(
            "{}: lists must have the same length ({} vs {})", ctx, a.len(), b.len()
        )))
    } else {
        Ok(())
    }
}

fn non_empty(v: &[f64], ctx: &str) -> VmResult<()> {
    if v.is_empty() {
        Err(VmError::new(format!("{}: list must not be empty", ctx)))
    } else {
        Ok(())
    }
}

// Size argument (rows, cols, n): an int >= 0. Negative values are an
// error instead of wrapping into a huge usize.
fn dim_arg(value: &Value, ctx: &str) -> VmResult<usize> {
    match value {
        Value::Int(n) if *n >= 0 => Ok(*n as usize),
        Value::Int(_) => Err(VmError::new(format!("{}: size must be >= 0", ctx))),
        _ => Err(VmError::new(format!("{}: size must be int", ctx))),
    }
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_dot(args: Vec<Value>) -> VmResult<Value> {
    expect_args("dot", &args, 2)?;
    let a = list_to_f64s(&args[0], "dot")?;
    let b = list_to_f64s(&args[1], "dot")?;
    same_len(&a, &b, "dot")?;
    Ok(Value::Float(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
}

fn native_norm(args: Vec<Value>) -> VmResult<Value> {
    expect_args("norm", &args, 1)?;
    let v = list_to_f64s(&args[0], "norm")?;
    Ok(Value::Float(v.iter().map(|x| x * x).sum::<f64>().sqrt()))
}

fn native_vec_add(args: Vec<Value>) -> VmResult<Value> {
    expect_args("vec_add", &args, 2)?;
    let a = list_to_f64s(&args[0], "vec_add")?;
    let b = list_to_f64s(&args[1], "vec_add")?;
    same_len(&a, &b, "vec_add")?;
    Ok(f64s_to_list(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()))
}

fn native_vec_sub(args: Vec<Value>) -> VmResult<Value> {
    expect_args("vec_sub", &args, 2)?;
    let a = list_to_f64s(&args[0], "vec_sub")?;
    let b = list_to_f64s(&args[1], "vec_sub")?;
    same_len(&a, &b, "vec_sub")?;
    Ok(f64s_to_list(a.iter().zip(b.iter()).map(|(x, y)| x - y).collect()))
}

fn native_vec_mul(args: Vec<Value>) -> VmResult<Value> {
    expect_args("vec_mul", &args, 2)?;
    let a = list_to_f64s(&args[0], "vec_mul")?;
    let b = list_to_f64s(&args[1], "vec_mul")?;
    same_len(&a, &b, "vec_mul")?;
    Ok(f64s_to_list(a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()))
}

fn native_vec_scale(args: Vec<Value>) -> VmResult<Value> {
    expect_args("vec_scale", &args, 2)?;
    let v      = list_to_f64s(&args[0], "vec_scale")?;
    let scalar = to_f64(&args[1], "vec_scale")?;
    Ok(f64s_to_list(v.iter().map(|x| x * scalar).collect()))
}

fn native_argmax(args: Vec<Value>) -> VmResult<Value> {
    expect_args("argmax", &args, 1)?;
    let v = list_to_f64s(&args[0], "argmax")?;
    non_empty(&v, "argmax")?;
    let idx = v.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    Ok(Value::Int(idx as i64))
}

fn native_matrix_new(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_new", &args, 3)?;
    let rows = dim_arg(&args[0], "matrix_new")?;
    let cols = dim_arg(&args[1], "matrix_new")?;
    let fill = to_f64(&args[2], "matrix_new")?;
    Ok(f64s_to_list(vec![fill; rows * cols]))
}

fn native_matrix_mul(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_mul", &args, 5)?;
    let a     = list_to_f64s(&args[0], "matrix_mul")?;
    let b     = list_to_f64s(&args[1], "matrix_mul")?;
    let rows  = dim_arg(&args[2], "matrix_mul")?;
    let inner = dim_arg(&args[3], "matrix_mul")?;
    let cols  = dim_arg(&args[4], "matrix_mul")?;
    if a.len() != rows * inner {
        return Err(VmError::new(format!("matrix_mul: A must have {} elements, got {}", rows * inner, a.len())));
    }
    if b.len() != inner * cols {
        return Err(VmError::new(format!("matrix_mul: B must have {} elements, got {}", inner * cols, b.len())));
    }
    let mut result = vec![0.0f64; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let mut sum = 0.0;
            for k in 0..inner { sum += a[r * inner + k] * b[k * cols + c]; }
            result[r * cols + c] = sum;
        }
    }
    Ok(f64s_to_list(result))
}

fn native_matrix_add(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_add", &args, 2)?;
    let a = list_to_f64s(&args[0], "matrix_add")?;
    let b = list_to_f64s(&args[1], "matrix_add")?;
    same_len(&a, &b, "matrix_add")?;
    Ok(f64s_to_list(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()))
}

fn native_transpose(args: Vec<Value>) -> VmResult<Value> {
    expect_args("transpose", &args, 3)?;
    let m    = list_to_f64s(&args[0], "transpose")?;
    let rows = dim_arg(&args[1], "transpose")?;
    let cols = dim_arg(&args[2], "transpose")?;
    if m.len() != rows * cols {
        return Err(VmError::new(format!("transpose: matrix must have {} elements, got {}", rows * cols, m.len())));
    }
    let mut result = vec![0.0f64; rows * cols];
    for r in 0..rows {
        for c in 0..cols { result[c * rows + r] = m[r * cols + c]; }
    }
    Ok(f64s_to_list(result))
}

fn native_normalize(args: Vec<Value>) -> VmResult<Value> {
    expect_args("normalize", &args, 1)?;
    let v = list_to_f64s(&args[0], "normalize")?;
    non_empty(&v, "normalize")?;
    let min   = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max   = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 { return Ok(f64s_to_list(vec![0.0; v.len()])); }
    Ok(f64s_to_list(v.iter().map(|x| (x - min) / range).collect()))
}

fn native_standardize(args: Vec<Value>) -> VmResult<Value> {
    expect_args("standardize", &args, 1)?;
    let v = list_to_f64s(&args[0], "standardize")?;
    non_empty(&v, "standardize")?;
    let n    = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let std  = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
    if std == 0.0 { return Ok(f64s_to_list(vec![0.0; v.len()])); }
    Ok(f64s_to_list(v.iter().map(|x| (x - mean) / std).collect()))
}

fn native_clip(args: Vec<Value>) -> VmResult<Value> {
    expect_args("clip", &args, 3)?;
    let v   = list_to_f64s(&args[0], "clip")?;
    let min = to_f64(&args[1], "clip")?;
    let max = to_f64(&args[2], "clip")?;
    if min.is_nan() || max.is_nan() || min > max {
        return Err(VmError::new("clip: min must be <= max"));
    }
    Ok(f64s_to_list(v.iter().map(|x| x.clamp(min, max)).collect()))
}

fn native_linspace(args: Vec<Value>) -> VmResult<Value> {
    expect_args("linspace", &args, 3)?;
    let start = to_f64(&args[0], "linspace")?;
    let end   = to_f64(&args[1], "linspace")?;
    let n     = dim_arg(&args[2], "linspace")?;
    if n == 0 { return Ok(f64s_to_list(vec![])); }
    if n == 1 { return Ok(f64s_to_list(vec![start])); }
    let step = (end - start) / (n - 1) as f64;
    Ok(f64s_to_list((0..n).map(|i| start + i as f64 * step).collect()))
}

fn native_arange(args: Vec<Value>) -> VmResult<Value> {
    expect_args("arange", &args, 3)?;
    let start = to_f64(&args[0], "arange")?;
    let end   = to_f64(&args[1], "arange")?;
    let step  = to_f64(&args[2], "arange")?;
    if step == 0.0 { return Err(VmError::new("arange: step must not be zero")); }
    let mut result = vec![];
    let mut x = start;
    while (step > 0.0 && x < end) || (step < 0.0 && x > end) {
        result.push(x);
        x += step;
    }
    Ok(f64s_to_list(result))
}

/// cosine_similarity(a, b) → float
/// dot(a, b) / (norm(a) * norm(b)).
/// Returns 0 if either vector is zero.
fn native_cosine_similarity(args: Vec<Value>) -> VmResult<Value> {
    expect_args("cosine_similarity", &args, 2)?;
    let a = list_to_f64s(&args[0], "cosine_similarity")?;
    let b = list_to_f64s(&args[1], "cosine_similarity")?;
    same_len(&a, &b, "cosine_similarity")?;
    non_empty(&a, "cosine_similarity")?;
    let dot_ab = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f64>();
    let norm_a = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    let denom  = norm_a * norm_b;
    Ok(Value::Float(if denom == 0.0 { 0.0 } else { dot_ab / denom }))
}

/// euclidean_dist(a, b) → float
/// L2 distance: sqrt(sum((a - b)^2)).
fn native_euclidean_dist(args: Vec<Value>) -> VmResult<Value> {
    expect_args("euclidean_dist", &args, 2)?;
    let a = list_to_f64s(&args[0], "euclidean_dist")?;
    let b = list_to_f64s(&args[1], "euclidean_dist")?;
    same_len(&a, &b, "euclidean_dist")?;
    let dist = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt();
    Ok(Value::Float(dist))
}

/// manhattan_dist(a, b) → float
/// L1 distance: sum(|a - b|).
fn native_manhattan_dist(args: Vec<Value>) -> VmResult<Value> {
    expect_args("manhattan_dist", &args, 2)?;
    let a = list_to_f64s(&args[0], "manhattan_dist")?;
    let b = list_to_f64s(&args[1], "manhattan_dist")?;
    same_len(&a, &b, "manhattan_dist")?;
    let dist = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum::<f64>();
    Ok(Value::Float(dist))
}
