// liphia_core_native/src/random.rs
//
// Pseudo-random numbers. Not cryptographically secure.
//
// Functions registered:
//   seed(n: int)                          -> null   fixes the sequence (reproducible runs)
//   rand_int(low: int, high: int)         -> int    uniform in [low, high)
//   rand_uniform(n: int, low, high)       -> list   n floats uniform in [low, high)
//   rand_normal(n: int, mean, std)        -> list   n floats from N(mean, std)
//   shuffle(list)                         -> list   new list, same elements, random order
//
// The generator is SplitMix64. Without seed() it starts from the system
// clock, so each run produces a different sequence; call seed(n) at the top
// of a program when reproducibility matters.
//
// The state is thread-local to this binary. Native packages (cdylib) have
// their own copies of any state they keep, so they never see this seed:
// a package that needs randomness should receive it from Liphia code.

use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

use crate::util::{expect_args, int_arg, list_value, num_arg};

pub fn register(vm: &mut VM) {
    vm.register_native("seed", native_seed);
    vm.register_native("rand_int", native_rand_int);
    vm.register_native("rand_uniform", native_rand_uniform);
    vm.register_native("rand_normal", native_rand_normal);
    vm.register_native("shuffle", native_shuffle);
}

// ── Generator ─────────────────────────────────────────────────────────────────

thread_local! {
    static STATE: Cell<u64> = Cell::new(clock_seed());
}

fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

fn next_u64() -> u64 {
    STATE.with(|s| {
        let z = s.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        s.set(z);
        let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    })
}

// Uniform float in [0, 1) built from the top 53 bits.
fn next_f64() -> f64 {
    (next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

// Uniform integer in [0, bound) without modulo bias: values from the
// incomplete last block of the u64 range are rejected and redrawn.
fn below(bound: u64) -> u64 {
    let zone = u64::MAX - (u64::MAX % bound);
    loop {
        let r = next_u64();
        if r < zone {
            return r % bound;
        }
    }
}

fn count_arg(value: &Value, name: &str) -> VmResult<usize> {
    let n = int_arg(value, name)?;
    if n < 0 {
        return Err(VmError::new(format!("{}(): n must be >= 0", name)));
    }
    Ok(n as usize)
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_seed(args: Vec<Value>) -> VmResult<Value> {
    expect_args("seed", &args, 1)?;
    let n = int_arg(&args[0], "seed")?;
    STATE.with(|s| s.set(n as u64));
    Ok(Value::Null)
}

// The span is computed in i128 so that ranges like [MIN, MAX) do not
// overflow before being handed to the bounded sampler.
fn native_rand_int(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_int", &args, 2)?;
    let low = int_arg(&args[0], "rand_int")?;
    let high = int_arg(&args[1], "rand_int")?;
    if low >= high {
        return Err(VmError::new("rand_int(): low must be less than high"));
    }
    let span = (high as i128 - low as i128) as u64;
    Ok(Value::Int((low as i128 + below(span) as i128) as i64))
}

fn native_rand_uniform(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_uniform", &args, 3)?;
    let n = count_arg(&args[0], "rand_uniform")?;
    let low = num_arg(&args[1], "rand_uniform")?;
    let high = num_arg(&args[2], "rand_uniform")?;
    if !(low < high) {
        return Err(VmError::new("rand_uniform(): low must be less than high"));
    }
    let items = (0..n)
        .map(|_| Value::Float(low + next_f64() * (high - low)))
        .collect();
    Ok(list_value(items))
}

// Box-Muller transform: each pair of uniforms yields two independent
// standard normals; the second one is dropped when n is odd.
fn native_rand_normal(args: Vec<Value>) -> VmResult<Value> {
    expect_args("rand_normal", &args, 3)?;
    let n = count_arg(&args[0], "rand_normal")?;
    let mean = num_arg(&args[1], "rand_normal")?;
    let std = num_arg(&args[2], "rand_normal")?;
    if std < 0.0 {
        return Err(VmError::new("rand_normal(): std must be >= 0"));
    }
    let mut items = Vec::with_capacity(n);
    while items.len() < n {
        let u1 = next_f64().max(f64::MIN_POSITIVE);
        let u2 = next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * std::f64::consts::PI * u2;
        items.push(Value::Float(mean + std * radius * angle.cos()));
        if items.len() < n {
            items.push(Value::Float(mean + std * radius * angle.sin()));
        }
    }
    Ok(list_value(items))
}

// Fisher-Yates over a copy: the original list is left untouched and any
// element type is allowed.
fn native_shuffle(args: Vec<Value>) -> VmResult<Value> {
    expect_args("shuffle", &args, 1)?;
    let mut items = match &args[0] {
        Value::List(rc) => rc.borrow().clone(),
        _ => return Err(VmError::new("shuffle(): argument must be a list")),
    };
    for i in (1..items.len()).rev() {
        let j = below(i as u64 + 1) as usize;
        items.swap(i, j);
    }
    Ok(list_value(items))
}
