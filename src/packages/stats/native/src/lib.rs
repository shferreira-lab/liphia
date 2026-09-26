// packages/stats/native/src/lib.rs
//
// Native library of the `stats` package: inferential statistics.
// Loaded by the VM through liphia_virtual_machine::external (see
// packages/stats/index.lph). Descriptive statistics live in `num`.
//
// Functions registered:
//   t_stat_independent(a, b)   -> float   Welch's t
//   t_degrees_of_freedom(a, b) -> float   Welch-Satterthwaite df
//   t_stat_paired(a, b)        -> float   paired t
//   mann_whitney_u(a, b)       -> float   U statistic (smaller of U1, U2)
//   wilcoxon_w(a, b)           -> float   signed-rank W (sum of positive ranks)
//   shapiro_wilk_w(v)          -> float   W statistic, n in [3, 50]
//   p_value_t_ind(a, b)        -> float   two-tailed, Welch t-test
//   p_value_t_paired(a, b)     -> float   two-tailed, paired t-test
//   p_value_normal(z)          -> float   two-tailed, standard normal
//   p_value_mann_whitney(a, b) -> float   two-tailed, normal approx, min(n1, n2) > 10
//   p_value_wilcoxon(a, b)     -> float   two-tailed, normal approx, > 20 non-zero diffs

mod cdf;

use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

// ABI tag checked by the VM loader before liphia_register_module is
// called; it embeds the VM version and rustc this library was built with.
liphia_virtual_machine::export_package_abi!();

// Entry point called by the loader right after dlopen. The signature must
// match liphia_virtual_machine::external's RegisterFn exactly.
#[no_mangle]
pub extern "C" fn liphia_register_module(vm: *mut VM) {
    // SAFETY: the loader passes a valid, non-null *mut VM for the duration
    // of this call.
    let vm = unsafe { &mut *vm };
    vm.register_native("t_stat_independent",     native_t_stat_independent);
    vm.register_native("t_degrees_of_freedom",   native_t_degrees_of_freedom);
    vm.register_native("t_stat_paired",          native_t_stat_paired);
    vm.register_native("mann_whitney_u",         native_mann_whitney_u);
    vm.register_native("wilcoxon_w",             native_wilcoxon_w);
    vm.register_native("shapiro_wilk_w",         native_shapiro_wilk_w);
    vm.register_native("p_value_t_ind",          native_p_value_t_ind);
    vm.register_native("p_value_t_paired",       native_p_value_t_paired);
    vm.register_native("p_value_normal",         native_p_value_normal);
    vm.register_native("p_value_mann_whitney",   native_p_value_mann_whitney);
    vm.register_native("p_value_wilcoxon",       native_p_value_wilcoxon);
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

// ── Natives ───────────────────────────────────────────────────────────────────

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

/// Two-tailed p-value for the Wilcoxon signed-rank test via normal
/// approximation. Zero differences are dropped first (Wilcoxon's method);
/// only reliable when more than 20 non-zero differences remain.
fn native_p_value_wilcoxon(args: Vec<Value>) -> VmResult<Value> {
    let (a, b) = extract_two_equal(&args, "p_value_wilcoxon")?;
    let nonzero: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).filter(|d| *d != 0.0).collect();
    let n = nonzero.len() as f64;
    if n <= 20.0 {
        return Err(VmError::new(
            "p_value_wilcoxon(): normal approximation requires more than 20 non-zero \
             differences; for small samples use wilcoxon_w() with a critical-value table"
        ));
    }
    let abs_v: Vec<f64> = nonzero.iter().map(|d| d.abs()).collect();
    let r = ranks(&abs_v);
    let w: f64 = nonzero.iter().zip(r.iter()).filter(|(d, _)| **d > 0.0).map(|(_, rk)| rk).sum();
    let mu = n * (n + 1.0) / 4.0;
    let sigma = (n * (n + 1.0) * (2.0 * n + 1.0) / 24.0).sqrt();
    let z = (w - mu) / sigma;
    Ok(Value::Float(2.0 * cdf::normal_sf(z.abs())))
}
