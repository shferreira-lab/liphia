// packages/num/native/src/describe.rs
//
// Descriptive statistics and correlation over lists of numbers.
//
// Functions registered:
//   median(v), variance(v), stdev(v) -> float          population (divide by n)
//   variance_sample(v), stdev_sample(v) -> float       sample (divide by n - 1)
//   percentile(v, p) -> float   linear interpolation, p in [0, 100]
//   iqr(v) -> float             Q3 - Q1
//   zscore(v) -> list           sample z-scores
//   covariance(x, y) -> float   sample covariance
//   mode(v) -> float            most frequent value
//   range_stat(v) -> float      max - min
//   pearson_r(x, y), spearman_r(x, y), kendall_tau(x, y) -> float
//
// sum, mean, min_list and max_list are core natives, not part of num.

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

pub fn register(vm: &mut VM) {
    vm.register_native("median",                 native_median);
    vm.register_native("variance",               native_variance);
    vm.register_native("stdev",                  native_stdev);
    vm.register_native("variance_sample",        native_variance_sample);
    vm.register_native("stdev_sample",           native_stdev_sample);
    vm.register_native("percentile",             native_percentile);
    vm.register_native("iqr",                    native_iqr);
    vm.register_native("zscore",                 native_zscore);
    vm.register_native("covariance",             native_covariance);
    vm.register_native("mode",                   native_mode);
    vm.register_native("range_stat",             native_range_stat);
    vm.register_native("pearson_r",              native_pearson_r);
    vm.register_native("spearman_r",             native_spearman_r);
    vm.register_native("kendall_tau",            native_kendall_tau);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn list_to_floats(v: &Value, fn_name: &str) -> VmResult<Vec<f64>> {
    match v {
        Value::List(rc) => {
            let items = rc.borrow();
            if items.is_empty() {
                return Err(VmError::new(format!("{}(): list must not be empty", fn_name)));
            }
            items.iter().map(|v| match v {
                Value::Int(i)   => Ok(*i as f64),
                Value::Float(f) => Ok(*f),
                _ => Err(VmError::new(format!(
                    "{}(): list must contain only int or float", fn_name
                ))),
            }).collect()
        }
        _ => Err(VmError::new(format!("{}(): argument must be a list", fn_name))),
    }
}

fn extract_one(args: &[Value], fn_name: &str) -> VmResult<Vec<f64>> {
    if args.len() != 1 {
        return Err(VmError::new(format!("{}() expects 1 argument (list)", fn_name)));
    }
    list_to_floats(&args[0], fn_name)
}

fn extract_two_equal(args: &[Value], fn_name: &str) -> VmResult<(Vec<f64>, Vec<f64>)> {
    if args.len() != 2 {
        return Err(VmError::new(format!("{}() expects 2 list arguments", fn_name)));
    }
    let x = list_to_floats(&args[0], fn_name)?;
    let y = list_to_floats(&args[1], fn_name)?;
    if x.len() != y.len() {
        return Err(VmError::new(format!(
            "{}(): both lists must have the same length ({} vs {})",
            fn_name, x.len(), y.len()
        )));
    }
    Ok((x, y))
}

fn mean_of(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn sample_var(v: &[f64]) -> f64 {
    let m = mean_of(v);
    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let n = v.len();
    let mut idx: Vec<(usize, f64)> = v.iter().cloned().enumerate().collect();
    idx.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut r = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && (idx[j].1 - idx[i].1).abs() < f64::EPSILON { j += 1; }
        let avg = (i + 1 + j) as f64 / 2.0;
        for k in i..j { r[idx[k].0] = avg; }
        i = j;
    }
    r
}

// ── Natives ───────────────────────────────────────────────────────────────────

fn native_median(args: Vec<Value>) -> VmResult<Value> {
    let mut v = extract_one(&args, "median")?;
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    let r = if v.len() % 2 == 0 { (v[mid-1] + v[mid]) / 2.0 } else { v[mid] };
    Ok(Value::Float(r))
}

fn native_variance(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "variance")?;
    let m = mean_of(&v);
    Ok(Value::Float(v.iter().map(|x| (x-m).powi(2)).sum::<f64>() / v.len() as f64))
}

fn native_stdev(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "stdev")?;
    let m = mean_of(&v);
    Ok(Value::Float((v.iter().map(|x| (x-m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()))
}

fn native_variance_sample(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "variance_sample")?;
    if v.len() < 2 { return Err(VmError::new("variance_sample(): needs >= 2 elements")); }
    Ok(Value::Float(sample_var(&v)))
}

fn native_stdev_sample(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "stdev_sample")?;
    if v.len() < 2 { return Err(VmError::new("stdev_sample(): needs >= 2 elements")); }
    Ok(Value::Float(sample_var(&v).sqrt()))
}

fn native_percentile(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 { return Err(VmError::new("percentile() expects (list, p)")); }
    let mut v = list_to_floats(&args[0], "percentile")?;
    let p = match &args[1] {
        Value::Int(i)   => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(VmError::new("percentile(): p must be numeric")),
    };
    if !(0.0..=100.0).contains(&p) {
        return Err(VmError::new("percentile(): p must be in [0, 100]"));
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 1 { return Ok(Value::Float(v[0])); }
    let idx = p / 100.0 * (n - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil()  as usize;
    Ok(Value::Float(v[lo] + (idx - lo as f64) * (v[hi] - v[lo])))
}

fn native_iqr(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 { return Err(VmError::new("iqr() expects 1 argument (list)")); }
    let v = args[0].clone();
    let q1 = match native_percentile(vec![v.clone(), Value::Float(25.0)])? {
        Value::Float(f) => f, _ => unreachable!()
    };
    let q3 = match native_percentile(vec![v, Value::Float(75.0)])? {
        Value::Float(f) => f, _ => unreachable!()
    };
    Ok(Value::Float(q3 - q1))
}

fn native_zscore(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 { return Err(VmError::new("zscore() expects 1 argument (list)")); }
    let v = list_to_floats(&args[0], "zscore")?;
    if v.len() < 2 { return Err(VmError::new("zscore(): needs >= 2 elements")); }
    let m  = mean_of(&v);
    let sd = sample_var(&v).sqrt();
    if sd == 0.0 { return Err(VmError::new("zscore(): standard deviation is zero")); }
    let zs: Vec<Value> = v.iter().map(|x| Value::Float((x - m) / sd)).collect();
    Ok(Value::List(std::rc::Rc::new(std::cell::RefCell::new(zs))))
}

fn native_covariance(args: Vec<Value>) -> VmResult<Value> {
    let (x, y) = extract_two_equal(&args, "covariance")?;
    if x.len() < 2 { return Err(VmError::new("covariance(): needs >= 2 elements")); }
    let mx = mean_of(&x);
    let my = mean_of(&y);
    let cov = x.iter().zip(y.iter())
        .map(|(xi, yi)| (xi - mx) * (yi - my))
        .sum::<f64>() / (x.len() - 1) as f64;
    Ok(Value::Float(cov))
}

fn native_mode(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "mode")?;
    use std::collections::HashMap;
    let mut counts: HashMap<u64, (f64, usize)> = HashMap::new();
    for &x in &v {
        let e = counts.entry(x.to_bits()).or_insert((x, 0));
        e.1 += 1;
    }
    let (val, _) = counts.values().max_by_key(|e| e.1)
        .ok_or_else(|| VmError::new("mode(): empty list"))?;
    Ok(Value::Float(*val))
}

fn native_range_stat(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "range_stat")?;
    let min = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Ok(Value::Float(max - min))
}

fn native_pearson_r(args: Vec<Value>) -> VmResult<Value> {
    let (x, y) = extract_two_equal(&args, "pearson_r")?;
    if x.len() < 2 { return Err(VmError::new("pearson_r(): needs >= 2 elements")); }
    let mx = mean_of(&x);
    let my = mean_of(&y);
    let num: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| (xi-mx)*(yi-my)).sum();
    let dx  = x.iter().map(|xi| (xi-mx).powi(2)).sum::<f64>().sqrt();
    let dy  = y.iter().map(|yi| (yi-my).powi(2)).sum::<f64>().sqrt();
    if dx == 0.0 || dy == 0.0 {
        return Err(VmError::new("pearson_r(): a list has zero variance"));
    }
    Ok(Value::Float(num / (dx * dy)))
}

fn native_spearman_r(args: Vec<Value>) -> VmResult<Value> {
    let (x, y) = extract_two_equal(&args, "spearman_r")?;
    if x.len() < 2 { return Err(VmError::new("spearman_r(): needs >= 2 elements")); }
    let rx = ranks(&x);
    let ry = ranks(&y);
    let mrx = mean_of(&rx);
    let mry = mean_of(&ry);
    let num: f64 = rx.iter().zip(ry.iter()).map(|(a, b)| (a-mrx)*(b-mry)).sum();
    let dx  = rx.iter().map(|a| (a-mrx).powi(2)).sum::<f64>().sqrt();
    let dy  = ry.iter().map(|b| (b-mry).powi(2)).sum::<f64>().sqrt();
    if dx == 0.0 || dy == 0.0 {
        return Err(VmError::new("spearman_r(): all ranks are identical"));
    }
    Ok(Value::Float(num / (dx * dy)))
}

fn native_kendall_tau(args: Vec<Value>) -> VmResult<Value> {
    let (x, y) = extract_two_equal(&args, "kendall_tau")?;
    let n = x.len();
    if n < 2 { return Err(VmError::new("kendall_tau(): needs >= 2 elements")); }

    let (mut conc, mut disc) = (0i64, 0i64);
    let (mut tie_x, mut tie_y) = (0i64, 0i64);

    for i in 0..n {
        for j in (i+1)..n {
            let dx = x[i] - x[j];
            let dy = y[i] - y[j];
            let s  = dx * dy;

            if s > 0.0      { conc += 1; }
            else if s < 0.0 { disc += 1; }
            if dx == 0.0 && dy != 0.0 { tie_x += 1; }
            if dy == 0.0 && dx != 0.0 { tie_y += 1; }
            
        }
    }

    let num = (conc - disc) as f64;
    let den = (((conc + disc + tie_x) * (conc + disc + tie_y)) as f64).sqrt();

    Ok(Value::Float(if den == 0.0 { 0.0 } else { num / den }))
}
