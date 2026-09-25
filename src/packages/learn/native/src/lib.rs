// packages/learn/native/src/lib.rs
//
// Native library of the `learn` package: machine learning primitives.
// Loaded by the VM through liphia_virtual_machine::external (see
// packages/learn/index.lph). Vector and matrix math lives in `num`.
//
// Functions registered:
//   sigmoid(x), relu(x), tanh_act(x), gelu(x), swish(x) -> float
//   leaky_relu(x, alpha), elu(x, alpha)                  -> float
//   softmax(v) -> list
//   mse(pred, target), mae(pred, target)                 -> float
//   cross_entropy(pred, target), binary_cross_entropy(pred, target) -> float
//   gradient_clip(grads, max_norm) -> list
//   sgd_update(weights, grads, lr) -> list
//   adam_update(weights, grads, m, v, t, lr, beta1, beta2, eps) -> list
//   accuracy, precision, recall, f1_score (pred, target) -> float
//     binary metrics; predictions >= 0.5 count as positive

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

// Entry point called by the loader right after dlopen. The signature must
// match liphia_virtual_machine::external's RegisterFn exactly.
#[no_mangle]
pub extern "C" fn liphia_register_module(vm: *mut VM) {
    // SAFETY: the loader passes a valid, non-null *mut VM for the duration
    // of this call.
    let vm = unsafe { &mut *vm };
    vm.register_native("sigmoid",                native_sigmoid);
    vm.register_native("relu",                   native_relu);
    vm.register_native("leaky_relu",             native_leaky_relu);
    vm.register_native("tanh_act",               native_tanh_act);
    vm.register_native("elu",                    native_elu);
    vm.register_native("gelu",                   native_gelu);
    vm.register_native("swish",                  native_swish);
    vm.register_native("softmax",                native_softmax);
    vm.register_native("mse",                    native_mse);
    vm.register_native("mae",                    native_mae);
    vm.register_native("cross_entropy",          native_cross_entropy);
    vm.register_native("binary_cross_entropy",   native_binary_cross_entropy);
    vm.register_native("gradient_clip",          native_gradient_clip);
    vm.register_native("sgd_update",             native_sgd_update);
    vm.register_native("adam_update",            native_adam_update);
    vm.register_native("accuracy",               native_accuracy);
    vm.register_native("precision",              native_precision);
    vm.register_native("recall",                 native_recall);
    vm.register_native("f1_score",               native_f1_score);
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

// ── Natives ───────────────────────────────────────────────────────────────────

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

fn native_softmax(args: Vec<Value>) -> VmResult<Value> {
    expect_args("softmax", &args, 1)?;
    let v = list_to_f64s(&args[0], "softmax")?;
    non_empty(&v, "softmax")?;
    let max  = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = v.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    Ok(f64s_to_list(exps.into_iter().map(|e| e / sum).collect()))
}

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
