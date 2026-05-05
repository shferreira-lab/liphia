// stdlib/native/src/ai.rs
//
// AI and machine learning primitives — pure Rust, no external dependencies.
// All matrix operations use flat Vec<f64> lists in row-major order.
//
// ── Functions registered ──────────────────────────────────────────────────────
//
// Activation functions:
//   sigmoid(x)                              → float
//   relu(x)                                 → float
//   leaky_relu(x, alpha)                    → float
//   tanh_act(x)                             → float
//   elu(x, alpha)                           → float
//   gelu(x)                                 → float
//   swish(x)                                → float
//
// Vector operations:
//   dot(a, b)                               → float
//   norm(v)                                 → float
//   vec_add(a, b)                           → list
//   vec_sub(a, b)                           → list
//   vec_mul(a, b)                           → list   Hadamard product
//   vec_scale(v, scalar)                    → list
//   vec_sum(v)                              → float
//
// Classification:
//   softmax(v)                              → list
//   argmax(v)                               → int
//
// Matrix operations (flat row-major):
//   matrix_new(rows, cols, fill)            → list
//   matrix_mul(a, b, rows, inner, cols)     → list
//   matrix_add(a, b)                        → list
//   transpose(m, rows, cols)                → list
//
// Data preprocessing:
//   normalize(v)                            → list
//   standardize(v)                          → list
//   clip(v, min, max)                       → list
//   linspace(start, end, n)                 → list
//   arange(start, end, step)                → list
//
// Loss functions:
//   mse(pred, target)                       → float
//   mae(pred, target)                       → float
//   cross_entropy(pred, target)             → float
//   binary_cross_entropy(pred, target)      → float
//
// Random:
//   seed(n)                                 → null
//   rand_uniform(n, low, high)              → list
//   rand_normal(n, mean, std)               → list
//   rand_int(low, high)                     → int
//   shuffle(v)                              → list
//
// Gradients and optimization:
//   gradient_clip(grads, max_norm)          → list
//   sgd_update(weights, grads, lr)          → list
//   adam_update(weights, grads, m, v, t, lr, beta1, beta2, eps) → list
//
// Classification metrics:
//   accuracy(pred, target)                  → float
//   precision(pred, target)                 → float
//   recall(pred, target)                    → float
//   f1_score(pred, target)                  → float
//
// Distance functions:
//   cosine_similarity(a, b)                 → float
//   euclidean_dist(a, b)                    → float
//   manhattan_dist(a, b)                    → float

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

// ── Registration ──────────────────────────────────────────────────────────────

pub fn register(vm: &mut VM) {
    // Activation functions
    vm.register_native("sigmoid",               native_sigmoid);
    vm.register_native("relu",                  native_relu);
    vm.register_native("leaky_relu",            native_leaky_relu);
    vm.register_native("tanh_act",              native_tanh_act);
    vm.register_native("elu",                   native_elu);
    vm.register_native("gelu",                  native_gelu);
    vm.register_native("swish",                 native_swish);
    // Vector operations
    vm.register_native("dot",                   native_dot);
    vm.register_native("norm",                  native_norm);
    vm.register_native("vec_add",               native_vec_add);
    vm.register_native("vec_sub",               native_vec_sub);
    vm.register_native("vec_mul",               native_vec_mul);
    vm.register_native("vec_scale",             native_vec_scale);
    vm.register_native("vec_sum",               native_vec_sum);
    // Classification
    vm.register_native("softmax",               native_softmax);
    vm.register_native("argmax",                native_argmax);
    // Matrix operations
    vm.register_native("matrix_new",            native_matrix_new);
    vm.register_native("matrix_mul",            native_matrix_mul);
    vm.register_native("matrix_add",            native_matrix_add);
    vm.register_native("transpose",             native_transpose);
    // Data preprocessing
    vm.register_native("normalize",             native_normalize);
    vm.register_native("standardize",           native_standardize);
    vm.register_native("clip",                  native_clip);
    vm.register_native("linspace",              native_linspace);
    vm.register_native("arange",                native_arange);
    // Loss functions
    vm.register_native("mse",                   native_mse);
    vm.register_native("mae",                   native_mae);
    vm.register_native("cross_entropy",         native_cross_entropy);
    vm.register_native("binary_cross_entropy",  native_binary_cross_entropy);
    // Random
    vm.register_native("seed",                  native_seed);
    vm.register_native("rand_uniform",          native_rand_uniform);
    vm.register_native("rand_normal",           native_rand_normal);
    vm.register_native("rand_int",              native_rand_int);
    vm.register_native("shuffle",               native_shuffle);
    // Gradients and optimization
    vm.register_native("gradient_clip",         native_gradient_clip);
    vm.register_native("sgd_update",            native_sgd_update);
    vm.register_native("adam_update",           native_adam_update);
    // Classification metrics
    vm.register_native("accuracy",              native_accuracy);
    vm.register_native("precision",             native_precision);
    vm.register_native("recall",                native_recall);
    vm.register_native("f1_score",              native_f1_score);
    // Distance functions
    vm.register_native("cosine_similarity",     native_cosine_similarity);
    vm.register_native("euclidean_dist",        native_euclidean_dist);
    vm.register_native("manhattan_dist",        native_manhattan_dist);
}

// ── RNG (LCG — no external crate) ────────────────────────────────────────────

thread_local! {
    static RNG_STATE: RefCell<u64> = RefCell::new(12345678901234567u64);
}

fn rng_next() -> u64 {
    RNG_STATE.with(|s| {
        let mut state = s.borrow_mut();
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    })
}

fn rng_f64() -> f64 {
    (rng_next() >> 11) as f64 / (1u64 << 53) as f64
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

// ── Activation functions ──────────────────────────────────────────────────────

fn native_sigmoid(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sigmoid", &args, 1)?;
    let x = to_f64(&args[0], "sigmoid")?;
    Ok(Value::Float(1.0 / (1.0 + (-x).exp())))
}

fn native_relu(args: Vec<Value>) -> VmResult<Value> {
    expect_args("relu", &args, 1)?;
    let x = to_f64(&args[0], "relu")?;
    Ok(Value::Float(if x > 0.0 { x } else { 0.0 }))
}

fn native_leaky_relu(args: Vec<Value>) -> VmResult<Value> {
    expect_args("leaky_relu", &args, 2)?;
    let x     = to_f64(&args[0], "leaky_relu")?;
    let alpha = to_f64(&args[1], "leaky_relu")?;
    Ok(Value::Float(if x >= 0.0 { x } else { alpha * x }))
}

fn native_tanh_act(args: Vec<Value>) -> VmResult<Value> {
    expect_args("tanh_act", &args, 1)?;
    let x = to_f64(&args[0], "tanh_act")?;
    Ok(Value::Float(x.tanh()))
}

fn native_elu(args: Vec<Value>) -> VmResult<Value> {
    expect_args("elu", &args, 2)?;
    let x     = to_f64(&args[0], "elu")?;
    let alpha = to_f64(&args[1], "elu")?;
    Ok(Value::Float(if x >= 0.0 { x } else { alpha * (x.exp() - 1.0) }))
}

fn native_gelu(args: Vec<Value>) -> VmResult<Value> {
    expect_args("gelu", &args, 1)?;
    let x   = to_f64(&args[0], "gelu")?;
    let sig = 1.0 / (1.0 + (-1.702 * x).exp());
    Ok(Value::Float(x * sig))
}

fn native_swish(args: Vec<Value>) -> VmResult<Value> {
    expect_args("swish", &args, 1)?;
    let x   = to_f64(&args[0], "swish")?;
    let sig = 1.0 / (1.0 + (-x).exp());
    Ok(Value::Float(x * sig))
}

// ── Vector operations ─────────────────────────────────────────────────────────

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

fn native_vec_sum(args: Vec<Value>) -> VmResult<Value> {
    expect_args("vec_sum", &args, 1)?;
    let v = list_to_f64s(&args[0], "vec_sum")?;
    Ok(Value::Float(v.iter().sum()))
}

// ── Classification ────────────────────────────────────────────────────────────

fn native_softmax(args: Vec<Value>) -> VmResult<Value> {
    expect_args("softmax", &args, 1)?;
    let v = list_to_f64s(&args[0], "softmax")?;
    non_empty(&v, "softmax")?;
    let max  = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = v.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    Ok(f64s_to_list(exps.into_iter().map(|e| e / sum).collect()))
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

// ── Matrix operations ─────────────────────────────────────────────────────────

fn native_matrix_new(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_new", &args, 3)?;
    let rows = match &args[0] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("matrix_new: rows must be int")) };
    let cols = match &args[1] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("matrix_new: cols must be int")) };
    let fill = to_f64(&args[2], "matrix_new")?;
    Ok(f64s_to_list(vec![fill; rows * cols]))
}

fn native_matrix_mul(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_mul", &args, 5)?;
    let a     = list_to_f64s(&args[0], "matrix_mul")?;
    let b     = list_to_f64s(&args[1], "matrix_mul")?;
    let rows  = match &args[2] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("matrix_mul: rows must be int")) };
    let inner = match &args[3] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("matrix_mul: inner must be int")) };
    let cols  = match &args[4] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("matrix_mul: cols must be int")) };
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
    let rows = match &args[1] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("transpose: rows must be int")) };
    let cols = match &args[2] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("transpose: cols must be int")) };
    if m.len() != rows * cols {
        return Err(VmError::new(format!("transpose: matrix must have {} elements, got {}", rows * cols, m.len())));
    }
    let mut result = vec![0.0f64; rows * cols];
    for r in 0..rows {
        for c in 0..cols { result[c * rows + r] = m[r * cols + c]; }
    }
    Ok(f64s_to_list(result))
}

// ── Data preprocessing ────────────────────────────────────────────────────────

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
    Ok(f64s_to_list(v.iter().map(|x| x.clamp(min, max)).collect()))
}

fn native_linspace(args: Vec<Value>) -> VmResult<Value> {
    expect_args("linspace", &args, 3)?;
    let start = to_f64(&args[0], "linspace")?;
    let end   = to_f64(&args[1], "linspace")?;
    let n     = match &args[2] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("linspace: n must be int")) };
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

// ── Loss functions ────────────────────────────────────────────────────────────

fn native_mse(args: Vec<Value>) -> VmResult<Value> {
    expect_args("mse", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "mse")?;
    let target = list_to_f64s(&args[1], "mse")?;
    same_len(&pred, &target, "mse")?;
    let loss = pred.iter().zip(target.iter()).map(|(p, t)| (p - t).powi(2)).sum::<f64>() / pred.len() as f64;
    Ok(Value::Float(loss))
}

fn native_mae(args: Vec<Value>) -> VmResult<Value> {
    expect_args("mae", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "mae")?;
    let target = list_to_f64s(&args[1], "mae")?;
    same_len(&pred, &target, "mae")?;
    let loss = pred.iter().zip(target.iter()).map(|(p, t)| (p - t).abs()).sum::<f64>() / pred.len() as f64;
    Ok(Value::Float(loss))
}

fn native_cross_entropy(args: Vec<Value>) -> VmResult<Value> {
    expect_args("cross_entropy", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "cross_entropy")?;
    let target = list_to_f64s(&args[1], "cross_entropy")?;
    same_len(&pred, &target, "cross_entropy")?;
    let eps  = 1e-12;
    let loss = pred.iter().zip(target.iter()).map(|(p, t)| -t * p.max(eps).ln()).sum::<f64>();
    Ok(Value::Float(loss))
}

fn native_binary_cross_entropy(args: Vec<Value>) -> VmResult<Value> {
    expect_args("binary_cross_entropy", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "binary_cross_entropy")?;
    let target = list_to_f64s(&args[1], "binary_cross_entropy")?;
    same_len(&pred, &target, "binary_cross_entropy")?;
    let eps  = 1e-12;
    let loss = pred.iter().zip(target.iter())
        .map(|(p, t)| {
            let p = p.clamp(eps, 1.0 - eps);
            -(t * p.ln() + (1.0 - t) * (1.0 - p).ln())
        })
        .sum::<f64>() / pred.len() as f64;
    Ok(Value::Float(loss))
}

// ── Random ────────────────────────────────────────────────────────────────────

fn native_seed(args: Vec<Value>) -> VmResult<Value> {
    expect_args("seed", &args, 1)?;
    let n = match &args[0] { Value::Int(n) => *n as u64, _ => return Err(VmError::new("seed: argument must be int")) };
    RNG_STATE.with(|s| *s.borrow_mut() = if n == 0 { 1 } else { n });
    Ok(Value::Null)
}

fn native_rand_uniform(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_uniform", &args, 3)?;
    let n    = match &args[0] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("rand_uniform: n must be int")) };
    let low  = to_f64(&args[1], "rand_uniform")?;
    let high = to_f64(&args[2], "rand_uniform")?;
    if low >= high { return Err(VmError::new("rand_uniform: low must be less than high")); }
    Ok(f64s_to_list((0..n).map(|_| low + rng_f64() * (high - low)).collect()))
}

fn native_rand_normal(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_normal", &args, 3)?;
    let n    = match &args[0] { Value::Int(n) => *n as usize, _ => return Err(VmError::new("rand_normal: n must be int")) };
    let mean = to_f64(&args[1], "rand_normal")?;
    let std  = to_f64(&args[2], "rand_normal")?;
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let u1 = rng_f64().max(1e-12);
        let u2 = rng_f64();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).sin();
        v.push(mean + z0 * std);
        if i + 1 < n { v.push(mean + z1 * std); }
        i += 2;
    }
    v.truncate(n);
    Ok(f64s_to_list(v))
}

fn native_rand_int(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_int", &args, 2)?;
    let low  = match &args[0] { Value::Int(n) => *n, _ => return Err(VmError::new("rand_int: low must be int")) };
    let high = match &args[1] { Value::Int(n) => *n, _ => return Err(VmError::new("rand_int: high must be int")) };
    if low >= high { return Err(VmError::new("rand_int: low must be less than high")); }
    let range = (high - low) as u64;
    Ok(Value::Int(low + (rng_next() % range) as i64))
}

fn native_shuffle(args: Vec<Value>) -> VmResult<Value> {
    expect_args("shuffle", &args, 1)?;
    let v = list_to_f64s(&args[0], "shuffle")?;
    let mut result = v.clone();
    let n = result.len();
    for i in (1..n).rev() {
        let j = (rng_next() as usize) % (i + 1);
        result.swap(i, j);
    }
    Ok(f64s_to_list(result))
}

// ── Gradients and optimization ────────────────────────────────────────────────

/// gradient_clip(grads, max_norm) → list
/// Scales the gradient vector so its L2 norm does not exceed max_norm.
/// If norm(grads) <= max_norm the gradients are returned unchanged.
fn native_gradient_clip(args: Vec<Value>) -> VmResult<Value> {
    expect_args("gradient_clip", &args, 2)?;
    let grads    = list_to_f64s(&args[0], "gradient_clip")?;
    let max_norm = to_f64(&args[1], "gradient_clip")?;
    if max_norm <= 0.0 { return Err(VmError::new("gradient_clip: max_norm must be > 0")); }
    let current_norm = grads.iter().map(|x| x * x).sum::<f64>().sqrt();
    if current_norm <= max_norm {
        return Ok(f64s_to_list(grads));
    }
    let scale = max_norm / current_norm;
    Ok(f64s_to_list(grads.iter().map(|x| x * scale).collect()))
}

/// sgd_update(weights, grads, lr) → list
/// Stochastic Gradient Descent: weights = weights - lr * grads
fn native_sgd_update(args: Vec<Value>) -> VmResult<Value> {
    expect_args("sgd_update", &args, 3)?;
    let weights = list_to_f64s(&args[0], "sgd_update")?;
    let grads   = list_to_f64s(&args[1], "sgd_update")?;
    let lr      = to_f64(&args[2], "sgd_update")?;
    same_len(&weights, &grads, "sgd_update")?;
    Ok(f64s_to_list(
        weights.iter().zip(grads.iter()).map(|(w, g)| w - lr * g).collect()
    ))
}

/// adam_update(weights, grads, m, v, t, lr, beta1, beta2, eps) → list
///
/// Adam optimizer — returns updated weights.
/// Standard defaults: lr=0.001, beta1=0.9, beta2=0.999, eps=1e-8
///
/// Note: m and v (moment vectors) must be maintained externally between steps.
/// t is the current step number (starts at 1).
///
/// Returns only the updated weights. Update m and v separately with:
///   m_new = vec_add(vec_scale(m, beta1), vec_scale(grads, 1 - beta1))
///   v_new = vec_add(vec_scale(v, beta2), vec_scale(vec_mul(grads, grads), 1 - beta2))
fn native_adam_update(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 9 {
        return Err(VmError::new(
            "adam_update(weights, grads, m, v, t, lr, beta1, beta2, eps) — expected 9 arguments"
        ));
    }
    let weights = list_to_f64s(&args[0], "adam_update")?;
    let grads   = list_to_f64s(&args[1], "adam_update")?;
    let m       = list_to_f64s(&args[2], "adam_update")?;
    let v       = list_to_f64s(&args[3], "adam_update")?;
    let t       = to_f64(&args[4], "adam_update")?;
    let lr      = to_f64(&args[5], "adam_update")?;
    let beta1   = to_f64(&args[6], "adam_update")?;
    let beta2   = to_f64(&args[7], "adam_update")?;
    let eps     = to_f64(&args[8], "adam_update")?;

    same_len(&weights, &grads, "adam_update")?;
    same_len(&weights, &m,     "adam_update")?;
    same_len(&weights, &v,     "adam_update")?;

    if t < 1.0 { return Err(VmError::new("adam_update: t must be >= 1")); }

    // Bias-corrected moment estimates
    let bc1 = 1.0 - beta1.powf(t);
    let bc2 = 1.0 - beta2.powf(t);

    let updated: Vec<f64> = weights.iter()
        .zip(grads.iter())
        .zip(m.iter())
        .zip(v.iter())
        .map(|(((w, g), mi), vi)| {
            let m_hat = (beta1 * mi + (1.0 - beta1) * g) / bc1;
            let v_hat = (beta2 * vi + (1.0 - beta2) * g * g) / bc2;
            w - lr * m_hat / (v_hat.sqrt() + eps)
        })
        .collect();

    Ok(f64s_to_list(updated))
}

// ── Classification metrics ────────────────────────────────────────────────────
//
// All metrics treat values >= 0.5 as positive (class 1), < 0.5 as negative (class 0).
// pred and target must be lists of the same length.
// target should contain 0.0 or 1.0 values.

fn threshold(x: f64) -> bool { x >= 0.5 }

fn confusion(pred: &[f64], target: &[f64]) -> (f64, f64, f64, f64) {
    let (mut tp, mut fp, mut tn, mut fn_) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (p, t) in pred.iter().zip(target.iter()) {
        let pp = threshold(*p);
        let pt = threshold(*t);
        match (pp, pt) {
            (true,  true)  => tp += 1.0,
            (true,  false) => fp += 1.0,
            (false, true)  => fn_ += 1.0,
            (false, false) => tn += 1.0,
        }
    }
    (tp, fp, tn, fn_)
}

/// accuracy(pred, target) → float
/// Fraction of correct predictions.
fn native_accuracy(args: Vec<Value>) -> VmResult<Value> {
    expect_args("accuracy", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "accuracy")?;
    let target = list_to_f64s(&args[1], "accuracy")?;
    same_len(&pred, &target, "accuracy")?;
    non_empty(&pred, "accuracy")?;
    let (tp, _, tn, _) = confusion(&pred, &target);
    Ok(Value::Float((tp + tn) / pred.len() as f64))
}

/// precision(pred, target) → float
/// TP / (TP + FP). Returns 0 if no positive predictions.
fn native_precision(args: Vec<Value>) -> VmResult<Value> {
    expect_args("precision", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "precision")?;
    let target = list_to_f64s(&args[1], "precision")?;
    same_len(&pred, &target, "precision")?;
    non_empty(&pred, "precision")?;
    let (tp, fp, _, _) = confusion(&pred, &target);
    let denom = tp + fp;
    Ok(Value::Float(if denom == 0.0 { 0.0 } else { tp / denom }))
}

/// recall(pred, target) → float
/// TP / (TP + FN). Returns 0 if no actual positives.
fn native_recall(args: Vec<Value>) -> VmResult<Value> {
    expect_args("recall", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "recall")?;
    let target = list_to_f64s(&args[1], "recall")?;
    same_len(&pred, &target, "recall")?;
    non_empty(&pred, "recall")?;
    let (tp, _, _, fn_) = confusion(&pred, &target);
    let denom = tp + fn_;
    Ok(Value::Float(if denom == 0.0 { 0.0 } else { tp / denom }))
}

/// f1_score(pred, target) → float
/// Harmonic mean of precision and recall: 2 * P * R / (P + R).
fn native_f1_score(args: Vec<Value>) -> VmResult<Value> {
    expect_args("f1_score", &args, 2)?;
    let pred   = list_to_f64s(&args[0], "f1_score")?;
    let target = list_to_f64s(&args[1], "f1_score")?;
    same_len(&pred, &target, "f1_score")?;
    non_empty(&pred, "f1_score")?;
    let (tp, fp, _, fn_) = confusion(&pred, &target);
    let p     = if tp + fp  == 0.0 { 0.0 } else { tp / (tp + fp)  };
    let r     = if tp + fn_ == 0.0 { 0.0 } else { tp / (tp + fn_) };
    let denom = p + r;
    Ok(Value::Float(if denom == 0.0 { 0.0 } else { 2.0 * p * r / denom }))
}

// ── Distance functions ────────────────────────────────────────────────────────

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