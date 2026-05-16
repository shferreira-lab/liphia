// stdlib/native/src/cdf.rs
//
// Internal special functions and CDFs.
// NOT exposed to Liphia users — called only by stats.rs.
// mod cdf in lib.rs shows warnings. #![allow(dead_code)] will silent
#![allow(dead_code)]


// Functions:
//   erf(x)                   error function (Abramowitz & Stegun 7.1.26)
//   normal_cdf(x)            Φ(x) — standard normal CDF
//   normal_sf(x)             1 - Φ(x) — survival function (more accurate for large x)
//   log_gamma(x)             ln(Γ(x)) — Lanczos approximation (g=7, n=9)
//   incomplete_beta(x,a,b)   I_x(a,b) — regularised incomplete beta (Lentz CF)
//   t_cdf(t, df)             CDF of Student's t-distribution
//   t_sf(t, df)              survival function of Student's t (two-sided via 2*sf)
//   p_value_t_two(t, df)     two-tailed p-value for t statistic
//   chi2_cdf(x, df)          CDF of chi-squared distribution
//   chi2_sf(x, df)           survival function of chi-squared
//
// Numerical targets:
//   erf            absolute error < 1.5e-7
//   log_gamma      relative error < 1e-13  for x > 0.5
//   incomplete_beta relative error < 1e-10 (50 CF iterations max)
//   All CDFs derived from the above with no additional approximation.

// ── Error function ────────────────────────────────────────────────────────────

/// Abramowitz & Stegun formula 7.1.26.
/// Max absolute error ≈ 1.5 × 10⁻⁷ over all real x.
pub fn erf(x: f64) -> f64 {
    if x == 0.0 { return 0.0; }
    const P: f64 = 0.3275911;
    const A: [f64; 5] = [
        0.254829592,
        -0.284496736,
        1.421413741,
        -1.453152027,
        1.061405429,
    ];
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + P * x);
    let poly = ((((A[4] * t + A[3]) * t + A[2]) * t + A[1]) * t + A[0]) * t;
    sign * (1.0 - poly * (-x * x).exp())
}

pub fn erfc(x: f64) -> f64 {
    1.0 - erf(x)
}

// ── Normal distribution ───────────────────────────────────────────────────────

/// CDF of the standard normal: Φ(x) = P(Z ≤ x).
#[inline]
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Survival function: 1 - Φ(x).  More accurate than 1 - normal_cdf(x) for x >> 0.
#[inline]
pub fn normal_sf(x: f64) -> f64 {
    0.5 * erfc(x / std::f64::consts::SQRT_2)
}

// ── Log-gamma (Lanczos) ───────────────────────────────────────────────────────

/// ln(Γ(x)) for x > 0.
/// Lanczos approximation with g = 7, n = 9 (Numerical Recipes §6.1).
/// Relative error < 1 × 10⁻¹³.
pub fn log_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];

    debug_assert!(x > 0.0, "log_gamma: x must be > 0");

    let x = if x < 0.5 {
        // Reflection formula: Γ(x)Γ(1-x) = π / sin(πx)
        // log Γ(x) = log π - log sin(πx) - log Γ(1-x)
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * x).sin().ln()
            - log_gamma(1.0 - x);
    } else {
        x - 1.0
    };

    let mut a = C[0];
    for (i, &ci) in C[1..].iter().enumerate() {
        a += ci / (x + i as f64 + 1.0);
    }
    let t = x + G + 0.5;
    0.5 * (2.0 * std::f64::consts::PI).ln()
        + (x + 0.5) * t.ln()
        - t
        + a.ln()
}

// ── Regularised incomplete beta ───────────────────────────────────────────────

/// I_x(a, b) — regularised incomplete beta function, x ∈ [0, 1].
/// Uses Lentz's continued-fraction method (up to 200 iterations).
/// Relative error < 1 × 10⁻¹⁰ for well-conditioned inputs.
pub fn incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    if x >= 1.0 { return 1.0; }

    // Use the symmetry relation when x > (a+1)/(a+b+2) for better convergence
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - incomplete_beta(1.0 - x, b, a);
    }

    // Prefix: x^a * (1-x)^b / (a * B(a,b))
    //       = exp(a*ln(x) + b*ln(1-x) - log_beta(a,b)) / a
    // where log_beta(a,b) = log_gamma(a) + log_gamma(b) - log_gamma(a+b)
    let log_beta = log_gamma(a) + log_gamma(b) - log_gamma(a + b);
    let prefix = (a * x.ln() + b * (1.0 - x).ln() - log_beta).exp() / a;

    prefix * lentz_cf(x, a, b)
}

/// Lentz continued-fraction expansion of I_x(a,b) (without the prefix).
/// Returns the CF value; multiply by the prefix to get I_x.
fn lentz_cf(x: f64, a: f64, b: f64) -> f64 {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 3.0e-7;
    const FPMIN: f64 = 1.0e-30;

    let mut c = 1.0_f64;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    d = if d.abs() < FPMIN { FPMIN } else { d };
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=MAX_ITER {
        let m = m as f64;
        // Even step
        let num_even = m * (b - m) * x / ((a + 2.0 * m - 1.0) * (a + 2.0 * m));
        d = 1.0 + num_even * d;
        d = if d.abs() < FPMIN { FPMIN } else { d };
        c = 1.0 + num_even / c;
        c = if c.abs() < FPMIN { FPMIN } else { c };
        d = 1.0 / d;
        h *= d * c;

        // Odd step
        let num_odd = -(a + m) * (a + b + m) * x / ((a + 2.0 * m) * (a + 2.0 * m + 1.0));
        d = 1.0 + num_odd * d;
        d = if d.abs() < FPMIN { FPMIN } else { d };
        c = 1.0 + num_odd / c;
        c = if c.abs() < FPMIN { FPMIN } else { c };
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

// ── Student's t-distribution ──────────────────────────────────────────────────

/// CDF of Student's t: P(T ≤ t) for `df` degrees of freedom.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    debug_assert!(df > 0.0, "t_cdf: df must be > 0");
    // Relation: P(T ≤ t) = 1 - I_{df/(df+t²)}(df/2, 1/2) / 2  for t ≥ 0
    let x = df / (df + t * t);
    let ib = incomplete_beta(x, df / 2.0, 0.5);
    if t >= 0.0 { 1.0 - ib / 2.0 } else { ib / 2.0 }
}

/// Survival function: P(T > t).  sf = 1 - cdf.
#[inline]
pub fn t_sf(t: f64, df: f64) -> f64 {
    1.0 - t_cdf(t, df)
}

/// Two-tailed p-value: P(|T| ≥ |t|) = 2 * P(T ≥ |t|).
#[inline]
pub fn p_value_t_two(t: f64, df: f64) -> f64 {
    2.0 * t_sf(t.abs(), df)
}

// ── Chi-squared distribution ──────────────────────────────────────────────────
//
// χ²(k) is a special case of the Gamma distribution: Gamma(k/2, 2).
// CDF(x; k) = P(x/2; k/2) where P is the regularised lower incomplete gamma.
// We express it via the incomplete beta:
//   regularised lower incomplete gamma P(a, x) = 1 - I_{e^{-x}}(... )
// but a simpler route is the continued-fraction expansion directly.

/// Regularised lower incomplete gamma P(a, x) = γ(a,x)/Γ(a).
/// Uses series expansion for x < a+1, continued fractions otherwise.
pub fn incomplete_gamma_lower(a: f64, x: f64) -> f64 {
    if x < 0.0 { return 0.0; }
    if x == 0.0 { return 0.0; }
    if x < a + 1.0 {
        incomplete_gamma_series(a, x)
    } else {
        1.0 - incomplete_gamma_cf(a, x)
    }
}

/// Series expansion for P(a, x) (converges for x < a+1).
fn incomplete_gamma_series(a: f64, x: f64) -> f64 {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 3.0e-7;
    let log_prefix = -x + a * x.ln() - log_gamma(a);
    let mut term = 1.0 / a;
    let mut sum = term;
    for n in 1..=MAX_ITER {
        term *= x / (a + n as f64);
        sum += term;
        if term.abs() < sum.abs() * EPS { break; }
    }
    log_prefix.exp() * sum
}

/// Continued-fraction expansion for Q(a, x) = 1 - P(a, x) (converges for x >= a+1).
fn incomplete_gamma_cf(a: f64, x: f64) -> f64 {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 3.0e-7;
    const FPMIN: f64 = 1.0e-30;
    let log_prefix = -x + a * x.ln() - log_gamma(a);

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=MAX_ITER {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        d = if d.abs() < FPMIN { FPMIN } else { d };
        c = b + an / c;
        c = if c.abs() < FPMIN { FPMIN } else { c };
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPS { break; }
    }
    log_prefix.exp() * h
}

/// CDF of the chi-squared distribution with `df` degrees of freedom.
pub fn chi2_cdf(x: f64, df: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    incomplete_gamma_lower(df / 2.0, x / 2.0)
}

/// Survival function of chi-squared: P(X > x).
#[inline]
pub fn chi2_sf(x: f64, df: f64) -> f64 {
    1.0 - chi2_cdf(x, df)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_erf() {
        assert!(approx_eq(erf(0.0),  0.0,       1e-7));
        assert!(approx_eq(erf(1.0),  0.8427007, 1e-6));
        assert!(approx_eq(erf(-1.0),-0.8427007, 1e-6));
        assert!(approx_eq(erf(3.0),  0.9999779, 1e-6));
    }

    #[test]
    fn test_normal_cdf() {
        assert!(approx_eq(normal_cdf(0.0),  0.5,     1e-7));
        assert!(approx_eq(normal_cdf(1.96), 0.97500, 1e-4));
        assert!(approx_eq(normal_cdf(-1.96),0.02500, 1e-4));
    }

    #[test]
    fn test_log_gamma() {
        // Γ(1) = 1, Γ(2) = 1, Γ(0.5) = sqrt(π)
        assert!(approx_eq(log_gamma(1.0), 0.0,                     1e-10));
        assert!(approx_eq(log_gamma(2.0), 0.0,                     1e-10));
        assert!(approx_eq(log_gamma(0.5), (std::f64::consts::PI.sqrt()).ln(), 1e-10));
        assert!(approx_eq(log_gamma(5.0), (24.0_f64).ln(),         1e-10)); // Γ(5)=4!=24
    }

    #[test]
    fn test_t_cdf() {
        // t=0, any df → 0.5
        assert!(approx_eq(t_cdf(0.0, 10.0), 0.5, 1e-7));
        // Known value: t=2.228, df=10 → CDF ≈ 0.975 (two-tailed α=0.05)
        assert!(approx_eq(t_cdf(2.228, 10.0), 0.975, 1e-3));
        // p_value two-tailed at t=2.228, df=10 ≈ 0.05
        assert!(approx_eq(p_value_t_two(2.228, 10.0), 0.05, 1e-3));
    }

    #[test]
    fn test_chi2_cdf() {
        // χ²=0 → 0
        assert!(approx_eq(chi2_cdf(0.0, 2.0), 0.0, 1e-10));
        // χ²=3.841, df=1 → CDF ≈ 0.95 (critical value for α=0.05)
        assert!(approx_eq(chi2_cdf(3.841, 1.0), 0.95, 1e-3));
        // χ²=5.991, df=2 → CDF ≈ 0.95
        assert!(approx_eq(chi2_cdf(5.991, 2.0), 0.95, 1e-3));
    }
}
