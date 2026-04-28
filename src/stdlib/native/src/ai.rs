// stdlib/native/src/ai.rs
//
// AI and machine learning primitives — pure Rust, no external dependencies.
// All matrix operations use flat Vec<f64> lists.
//
// Functions registered:
//   sigmoid(x)                            → float
//   relu(x)                               → float
//   softmax(v: list)                      → list
//   argmax(v: list)                       → int
//   dot(a: list, b: list)                 → float
//   norm(v: list)                         → float
//   matrix_new(rows, cols, fill)          → list
//   matrix_mul(a, b, rows, inner, cols)   → list
//   matrix_add(a, b)                      → list

use std::cell::RefCell;
use std::rc::Rc;

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;

pub fn register(vm: &mut VM) {
    vm.register_native("sigmoid",    native_sigmoid);
    vm.register_native("relu",       native_relu);
    vm.register_native("softmax",    native_softmax);
    vm.register_native("argmax",     native_argmax);
    vm.register_native("dot",        native_dot);
    vm.register_native("norm",       native_norm);
    vm.register_native("matrix_new", native_matrix_new);
    vm.register_native("matrix_mul", native_matrix_mul);
    vm.register_native("matrix_add", native_matrix_add);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_f64(v: &Value, ctx: &str) -> VmResult<f64> {
    match v {
        Value::Int(i)   => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(VmError::new(format!("{}: argument must be int or float", ctx))),
    }
}

fn list_to_f64s(v: &Value, ctx: &str) -> VmResult<Vec<f64>> {
    match v {
        Value::List(rc) => {
            rc.borrow().iter().map(|x| to_f64(x, ctx)).collect()
        }
        _ => Err(VmError::new(format!("{}: argument must be a list", ctx))),
    }
}

fn f64s_to_list(v: Vec<f64>) -> Value {
    Value::List(Rc::new(RefCell::new(
        v.into_iter().map(Value::Float).collect()
    )))
}

fn expect_args(name: &str, args: &[Value], n: usize) -> VmResult<()> {
    if args.len() != n {
        Err(VmError::new(format!("{}() expects {} argument(s), got {}", name, n, args.len())))
    } else { Ok(()) }
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

fn native_softmax(args: Vec<Value>) -> VmResult<Value> {
    expect_args("softmax", &args, 1)?;
    let v = list_to_f64s(&args[0], "softmax")?;
    if v.is_empty() {
        return Err(VmError::new("softmax(): list must not be empty"));
    }

    // Garantir todos elementos são f64
    let v: Vec<f64> = v.into_iter().map(|x| x as f64).collect();

    let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = v.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    Ok(f64s_to_list(exps.into_iter().map(|e| e / sum).collect()))
}

fn native_argmax(args: Vec<Value>) -> VmResult<Value> {
    expect_args("argmax", &args, 1)?;
    let v = list_to_f64s(&args[0], "argmax")?;
    if v.is_empty() {
        return Err(VmError::new("argmax(): list must not be empty"));
    }

    let v: Vec<f64> = v.into_iter().map(|x| x as f64).collect();

    let idx = v.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    Ok(Value::Int(idx as i64))
}




// ── Linear algebra ────────────────────────────────────────────────────────────

fn native_dot(args: Vec<Value>) -> VmResult<Value> {
    expect_args("dot", &args, 2)?;
    let a = list_to_f64s(&args[0], "dot")?;
    let b = list_to_f64s(&args[1], "dot")?;
    if a.len() != b.len() {
        return Err(VmError::new(format!(
            "dot(): vectors must have the same length ({} vs {})", a.len(), b.len()
        )));
    }
    Ok(Value::Float(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
}

fn native_norm(args: Vec<Value>) -> VmResult<Value> {
    expect_args("norm", &args, 1)?;
    let v = list_to_f64s(&args[0], "norm")?;
    Ok(Value::Float(v.iter().map(|x| x * x).sum::<f64>().sqrt()))
}

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
        return Err(VmError::new(format!("matrix_mul: matrix A should have {} elements, got {}", rows * inner, a.len())));
    }
    if b.len() != inner * cols {
        return Err(VmError::new(format!("matrix_mul: matrix B should have {} elements, got {}", inner * cols, b.len())));
    }

    let mut result = vec![0.0f64; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let mut sum = 0.0;
            for k in 0..inner {
                sum += a[r * inner + k] * b[k * cols + c];
            }
            result[r * cols + c] = sum;
        }
    }
    Ok(f64s_to_list(result))
}

fn native_matrix_add(args: Vec<Value>) -> VmResult<Value> {
    expect_args("matrix_add", &args, 2)?;
    let a = list_to_f64s(&args[0], "matrix_add")?;
    let b = list_to_f64s(&args[1], "matrix_add")?;
    if a.len() != b.len() {
        return Err(VmError::new(format!(
            "matrix_add(): lists must have the same length ({} vs {})", a.len(), b.len()
        )));
    }
    Ok(f64s_to_list(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()))
}
