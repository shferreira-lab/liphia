// stdlib/native/src/stats.rs
//
// Statistical functions operating on Liphia lists.
//
// ── Original ──────────────────────────────────────────────────────────────────
//   sum(list)                  → float
//   mean(list)                 → float
//   min_list(list)             → float
//   max_list(list)             → float
//   median(list)               → float
//   variance(list)             → float   population (divides by n)
//   stdev(list)                → float   population
//   count(list)                → int
//
// ── Sample statistics (Bessel-corrected, n-1) ────────────────────────────────
//   variance_sample(list)      → float
//   stdev_sample(list)         → float
//
// ── Descriptive / shape ──────────────────────────────────────────────────────
//   percentile(list, p)        → float   linear interpolation, p ∈ [0,100]
//   iqr(list)                  → float   Q3 - Q1
//   zscore(list)               → list    sample z-normalisation
//   covariance(x, y)           → float   sample covariance
//   mode(list)                 → float   most frequent value
//   range_stat(list)           → float   max - min
//
// ── Correlation ───────────────────────────────────────────────────────────────
//   pearson_r(x, y)            → float   Pearson r ∈ [-1, 1]
//   spearman_r(x, y)           → float   Spearman ρ ∈ [-1, 1]
//   kendall_tau(x, y)          → float   Kendall τ ∈ [-1, 1]
//
// ── Two-sample test statistics (no CDF required) ─────────────────────────────
//   t_stat_independent(a, b)   → float   Welch's t
//   t_degrees_of_freedom(a, b) → float   Welch–Satterthwaite df
//   t_stat_paired(a, b)        → float   paired t
//   mann_whitney_u(a, b)       → float   U statistic (smaller of U1, U2)
//   wilcoxon_w(a, b)           → float   signed-rank W
//
// ── Normality ─────────────────────────────────────────────────────────────────
//   shapiro_wilk_w(list)       → float   W statistic, n ∈ [3, 50]
//
// ── p-values (via cdf.rs) ────────────────────────────────────────────────────
//   p_value_t_ind(a, b)        → float   two-tailed p, independent Welch t-test
//   p_value_t_paired(a, b)     → float   two-tailed p, paired t-test
//   p_value_normal(z)          → float   two-tailed p, standard normal
//   p_value_mann_whitney(a, b) → float   two-tailed p, Mann-Whitney (normal approx)
//                                         requires min(n1, n2) > 10

use liphia_virtual_machine::vm::{VM, VmError, VmResult};
use liphia_virtual_machine::value::Value;
use crate::cdf;

pub fn register(vm: &mut VM) {
    // ── Original ──────────────────────────────────────────────────────────────
    vm.register_native("sum",                      native_sum);
    vm.register_native("mean",                     native_mean);
    vm.register_native("min_list",                 native_min_list);
    vm.register_native("max_list",                 native_max_list);
    vm.register_native("median",                   native_median);
    vm.register_native("variance",                 native_variance);
    vm.register_native("stdev",                    native_stdev);
    vm.register_native("count",                    native_count);
    // ── Sample statistics ─────────────────────────────────────────────────────
    vm.register_native("variance_sample",          native_variance_sample);
    vm.register_native("stdev_sample",             native_stdev_sample);
    // ── Descriptive ───────────────────────────────────────────────────────────
    vm.register_native("percentile",               native_percentile);
    vm.register_native("iqr",                      native_iqr);
    vm.register_native("zscore",                   native_zscore);
    vm.register_native("covariance",               native_covariance);
    vm.register_native("mode",                     native_mode);
    vm.register_native("range_stat",               native_range_stat);
    // ── Correlation ───────────────────────────────────────────────────────────
    vm.register_native("pearson_r",                native_pearson_r);
    vm.register_native("spearman_r",               native_spearman_r);
    vm.register_native("kendall_tau",              native_kendall_tau);
    // ── Test statistics ───────────────────────────────────────────────────────
    vm.register_native("t_stat_independent",       native_t_stat_independent);
    vm.register_native("t_degrees_of_freedom",     native_t_degrees_of_freedom);
    vm.register_native("t_stat_paired",            native_t_stat_paired);
    vm.register_native("mann_whitney_u",           native_mann_whitney_u);
    vm.register_native("wilcoxon_w",               native_wilcoxon_w);
    // ── Normality ─────────────────────────────────────────────────────────────
    vm.register_native("shapiro_wilk_w",           native_shapiro_wilk_w);
    // ── p-values ──────────────────────────────────────────────────────────────
    vm.register_native("p_value_t_ind",            native_p_value_t_ind);
    vm.register_native("p_value_t_paired",         native_p_value_t_paired);
    vm.register_native("p_value_normal",           native_p_value_normal);
    vm.register_native("p_value_mann_whitney",     native_p_value_mann_whitney);
}

// ═════════════════════════════════════════════════════════════════════════════
// Internal helpers
// ═════════════════════════════════════════════════════════════════════════════

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

fn extract_two_any(args: &[Value], fn_name: &str) -> VmResult<(Vec<f64>, Vec<f64>)> {
    if args.len() != 2 {
        return Err(VmError::new(format!("{}() expects 2 list arguments", fn_name)));
    }
    Ok((list_to_floats(&args[0], fn_name)?, list_to_floats(&args[1], fn_name)?))
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

fn welch_t_df(a: &[f64], b: &[f64]) -> (f64, f64) {
    let na = a.len() as f64;
    let nb = b.len() as f64;
    let va = sample_var(a);
    let vb = sample_var(b);
    let se = (va / na + vb / nb).sqrt();
    let t  = if se == 0.0 { 0.0 } else { (mean_of(a) - mean_of(b)) / se };
    let num = (va / na + vb / nb).powi(2);
    let den = (va / na).powi(2) / (na - 1.0) + (vb / nb).powi(2) / (nb - 1.0);
    let df  = if den == 0.0 { na + nb - 2.0 } else { num / den };
    (t, df)
}

// ═════════════════════════════════════════════════════════════════════════════
// Original
// ═════════════════════════════════════════════════════════════════════════════

fn native_sum(args: Vec<Value>) -> VmResult<Value> {
    Ok(Value::Float(extract_one(&args, "sum")?.iter().sum()))
}
fn native_mean(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "mean")?;
    Ok(Value::Float(mean_of(&v)))
}
fn native_min_list(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "min_list")?;
    Ok(Value::Float(v.iter().cloned().fold(f64::INFINITY, f64::min)))
}
fn native_max_list(args: Vec<Value>) -> VmResult<Value> {
    let v = extract_one(&args, "max_list")?;
    Ok(Value::Float(v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
}
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
fn native_count(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 { return Err(VmError::new("count() expects 1 argument")); }
    match &args[0] {
        Value::List(rc) => Ok(Value::Int(rc.borrow().len() as i64)),
        _ => Err(VmError::new("count(): argument must be a list")),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Sample statistics
// ═════════════════════════════════════════════════════════════════════════════

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

// ═════════════════════════════════════════════════════════════════════════════
// Descriptive
// ═════════════════════════════════════════════════════════════════════════════

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

// ═════════════════════════════════════════════════════════════════════════════
// Correlation
// ═════════════════════════════════════════════════════════════════════════════

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


// ═════════════════════════════════════════════════════════════════════════════
// Two-sample test statistics
// ═════════════════════════════════════════════════════════════════════════════

fn native_t_stat_independent(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_any(&args, "t_stat_independent")?;
    if a.len() < 2 || b.len() < 2 {
        return Err(VmError::new("t_stat_independent(): each group needs >= 2 elements"));
    }
    Ok(Value::Float(welch_t_df(&a, &b).0))
}

fn native_t_degrees_of_freedom(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_any(&args, "t_degrees_of_freedom")?;
    if a.len() < 2 || b.len() < 2 {
        return Err(VmError::new("t_degrees_of_freedom(): each group needs >= 2 elements"));
    }
    Ok(Value::Float(welch_t_df(&a, &b).1))
}

fn native_t_stat_paired(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_equal(&args, "t_stat_paired")?;
    if a.len() < 2 { return Err(VmError::new("t_stat_paired(): needs >= 2 elements")); }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(ai, bi)| ai - bi).collect();
    let md = mean_of(&diffs);
    let n  = diffs.len() as f64;
    let sd = sample_var(&diffs).sqrt();
    Ok(Value::Float(if sd == 0.0 { 0.0 } else { md / (sd / n.sqrt()) }))
}

fn native_mann_whitney_u(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_any(&args, "mann_whitney_u")?;
    if a.is_empty() || b.is_empty() {
        return Err(VmError::new("mann_whitney_u(): lists must not be empty"));
    }
    let na = a.len() as f64;
    let nb = b.len() as f64;
    let mut u1 = 0.0f64;
    for &ai in &a {
        for &bj in &b {
            if ai > bj       { u1 += 1.0; }
            else if ai == bj { u1 += 0.5; }
        }
    }
    Ok(Value::Float(u1.min(na * nb - u1)))
}

fn native_wilcoxon_w(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_equal(&args, "wilcoxon_w")?;
    if a.len() < 2 { return Err(VmError::new("wilcoxon_w(): needs >= 2 elements")); }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(ai, bi)| ai - bi).collect();
    let nonzero: Vec<f64> = diffs.iter().cloned().filter(|&d| d != 0.0).collect();
    if nonzero.is_empty() { return Err(VmError::new("wilcoxon_w(): all differences are zero")); }
    let abs_v: Vec<f64> = nonzero.iter().map(|d| d.abs()).collect();
    let r = ranks(&abs_v);
    let w: f64 = nonzero.iter().zip(r.iter())
        .filter(|(&d, _)| d > 0.0)
        .map(|(_, &rk)| rk)
        .sum();
    Ok(Value::Float(w))
}

// ═════════════════════════════════════════════════════════════════════════════
// Normality
// ═════════════════════════════════════════════════════════════════════════════

fn native_shapiro_wilk_w(args: Vec<Value>) -> VmResult<Value> {
    let mut v = extract_one(&args, "shapiro_wilk_w")?;
    let n = v.len();
    if n < 3  { return Err(VmError::new("shapiro_wilk_w(): needs >= 3 elements")); }
    if n > 50 { return Err(VmError::new("shapiro_wilk_w(): approximate impl supports n <= 50")); }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    fn probit(p: f64) -> f64 {
        let p = p.clamp(1e-10, 1.0 - 1e-10);
        let t = if p <= 0.5 { (-2.0*p.ln()).sqrt() } else { (-2.0*(1.0-p).ln()).sqrt() };
        let c = [2.515517_f64, 0.802853, 0.010328];
        let d = [1.432788_f64, 0.189269, 0.001308];
        let z = t - (c[0]+c[1]*t+c[2]*t*t) / (1.0+d[0]*t+d[1]*t*t+d[2]*t*t*t);
        if p <= 0.5 { -z } else { z }
    }

    let m: Vec<f64> = (1..=n)
        .map(|i| probit((i as f64 - 0.375) / (n as f64 + 0.25)))
        .collect();
    let m_norm = m.iter().map(|x| x*x).sum::<f64>().sqrt();
    let a: Vec<f64> = m.iter().map(|x| x / m_norm).collect();
    let b: f64 = a.iter().zip(v.iter()).map(|(ai, xi)| ai*xi).sum();
    let xbar = mean_of(&v);
    let ss: f64 = v.iter().map(|xi| (xi-xbar).powi(2)).sum();
    if ss == 0.0 { return Err(VmError::new("shapiro_wilk_w(): all values are identical")); }
    Ok(Value::Float((b*b/ss).min(1.0)))
}

// ═════════════════════════════════════════════════════════════════════════════
// p-values (via cdf.rs)
// ═════════════════════════════════════════════════════════════════════════════

fn native_p_value_t_ind(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_any(&args, "p_value_t_ind")?;
    if a.len() < 2 || b.len() < 2 {
        return Err(VmError::new("p_value_t_ind(): each group needs >= 2 elements"));
    }
    let (t, df) = welch_t_df(&a, &b);
    Ok(Value::Float(cdf::p_value_t_two(t, df)))
}

fn native_p_value_t_paired(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_equal(&args, "p_value_t_paired")?;
    if a.len() < 2 { return Err(VmError::new("p_value_t_paired(): needs >= 2 elements")); }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(ai, bi)| ai - bi).collect();
    let md = mean_of(&diffs);
    let n  = diffs.len() as f64;
    let sd = sample_var(&diffs).sqrt();
    let t  = if sd == 0.0 { 0.0 } else { md / (sd / n.sqrt()) };
    Ok(Value::Float(cdf::p_value_t_two(t, n - 1.0)))
}

fn native_p_value_normal(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 {
        return Err(VmError::new("p_value_normal() expects 1 argument (z: float)"));
    }
    let z = match &args[0] {
        Value::Int(i)   => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(VmError::new("p_value_normal(): argument must be numeric")),
    };
    Ok(Value::Float(2.0 * cdf::normal_sf(z.abs())))
}

/// Two-tailed p-value for Mann-Whitney U via normal approximation.
/// Only reliable when min(n1, n2) > 10.
fn native_p_value_mann_whitney(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_any(&args, "p_value_mann_whitney")?;
    if a.is_empty() || b.is_empty() {
        return Err(VmError::new("p_value_mann_whitney(): lists must not be empty"));
    }
    let na = a.len() as f64;
    let nb = b.len() as f64;
    if na.min(nb) <= 10.0 {
        return Err(VmError::new(
            "p_value_mann_whitney(): normal approximation requires min(n1,n2) > 10; \
             for small samples use mann_whitney_u() with a critical-value table"
        ));
    }
    let mut u1 = 0.0f64;
    for &ai in &a {
        for &bj in &b {
            if ai > bj       { u1 += 1.0; }
            else if ai == bj { u1 += 0.5; }
        }
    }
    let u     = u1.min(na * nb - u1);
    let mu    = na * nb / 2.0;
    let sigma = (na * nb * (na + nb + 1.0) / 12.0).sqrt();
    let z     = (u - mu + 0.5) / sigma;   // continuity correction
    Ok(Value::Float(2.0 * cdf::normal_sf(z.abs())))
}
