# Changelog

All notable changes to the Liphia engine are documented here.

---

## [Unreleased] — stdlib · 0.2.1


### Added

**`http` — 1.1.0** (stdlib/native/src/http.rs)

Native CORS support added. All HTTP responses now automatically include the following headers:

- `Access-Control-Allow-Origin: *`
- `Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS`
- `Access-Control-Allow-Headers: Content-Type, Authorization, X-Requested-With`
- `Access-Control-Max-Age: 86400`

These headers are injected by `send_response` — the internal function called by both
`http_respond` and `http_respond_json` — so CORS is handled automatically for every
response without any change to user `.lph` code.

To support browser preflight requests (`OPTIONS`), add an `OPTIONS` handler to your router:

```lph
fn route() -> void:
    var method: str = http_method()
    var path:   str = http_path()

    if method == "OPTIONS":
        http_respond_json(200, "{}")
    else:
        if method == "POST" and path == "/login":
            handle_login()
        else:
            http_respond_json(404, "{\"error\": \"not found\"}")
```

No new functions were added in this release. The public API surface is unchanged:

```
http_listen, http_accept, http_method, http_path, http_query,
http_body, http_header, http_respond, http_respond_json,
http_get, http_post, http_put, http_patch, http_delete, http_status
```

**`stdlib/lph/http.lph` — 1.0.2**

Documentation updated to reflect native CORS support and the `OPTIONS` preflight pattern.

---

**`math` — 1.0.2** (stdlib/native/src/math.rs)

20 new functions added on top of the original 15.

- Inverse trigonometry: `asin`, `acos`, `atan`, `atan2`
  - `asin` and `acos` validate domain `[-1, 1]` and return a runtime error on violation
  - `atan2` is four-quadrant, correctly handles `x = 0`
- Hyperbolic functions: `sinh`, `cosh`, `tanh`
- Exponential / logarithm: `exp`, `log2`, `log_base(x, b)`
  - `log_base` accepts any positive base `b ≠ 1`; computed as `ln(x)/ln(b)`
- Number theory / combinatorics: `factorial(n)`, `gcd(a, b)`, `lcm(a, b)`
  - `factorial` is bounded to `n ≤ 20` to prevent `i64` overflow (21! overflows)
  - `gcd` uses the Euclidean algorithm and accepts negative inputs
- Geometry: `hypot(a, b)`, `deg_to_rad(x)`, `rad_to_deg(x)`
  - `hypot` delegates to Rust's `f64::hypot` for numerical stability
- Utilities: `sign(x)`, `clamp(x, lo, hi)`, `is_nan(x)`, `is_inf(x)`
  - `clamp` preserves the input type when all three arguments are the same type; promotes to `float` on mixed types
  - `is_nan` and `is_inf` always return `false` for integer inputs

**`stats` — 1.0.3** (stdlib/native/src/stats.rs + new cdf.rs)

18 new functions added on top of the original 8. Internal CDF module added (not user-visible).

*Sample statistics (Bessel-corrected, divide by n-1):*
- `variance_sample(list)` — sample variance
- `stdev_sample(list)` — sample standard deviation

*Descriptive / shape:*
- `percentile(list, p)` — p-th percentile via linear interpolation (same as NumPy default)
- `iqr(list)` — interquartile range (Q3 − Q1)
- `zscore(list)` — returns a new list of z-scores using sample standard deviation
- `covariance(x, y)` — sample covariance of two equal-length lists
- `mode(list)` — most frequent value; on ties returns the first found
- `range_stat(list)` — max − min

*Correlation:*
- `pearson_r(x, y)` — Pearson product-moment correlation; result ∈ [-1, 1]
- `spearman_r(x, y)` — Spearman rank correlation; handles ties by averaging ranks
- `kendall_tau(x, y)` — Kendall τ-a; more robust than Spearman for small samples or many ties

*Two-sample test statistics (no CDF required):*
- `t_stat_independent(a, b)` — Welch's t statistic for independent groups (unequal variance)
- `t_degrees_of_freedom(a, b)` — Welch–Satterthwaite degrees of freedom
- `t_stat_paired(a, b)` — paired t statistic; `df = n - 1`
- `mann_whitney_u(a, b)` — Mann-Whitney U (smaller of U1, U2); non-parametric
- `wilcoxon_w(a, b)` — Wilcoxon signed-rank W; zero differences excluded from ranking

*Normality:*
- `shapiro_wilk_w(list)` — Shapiro-Wilk W statistic; approximate, reliable for n ∈ [3, 50]

*p-values (via internal CDF module):*
- `p_value_t_ind(a, b)` — two-tailed p-value for Welch independent t-test
- `p_value_t_paired(a, b)` — two-tailed p-value for paired t-test
- `p_value_normal(z)` — two-tailed p-value under the standard normal: `2·(1−Φ(|z|))`
- `p_value_mann_whitney(a, b)` — two-tailed p-value for Mann-Whitney via normal approximation (requires `min(n1, n2) > 10`)

**`cdf.rs` — internal module** (stdlib/native/src/cdf.rs)

Pure-Rust special functions powering all p-value computations. Not registered in the VM and not accessible to Liphia user code. Declared as `mod cdf` (private) in `lib.rs`.

- `erf(x)` — error function; Abramowitz & Stegun 7.1.26, max absolute error < 1.5×10⁻⁷
- `erfc(x)` — complementary error function
- `normal_cdf(x)` — standard normal CDF Φ(x)
- `normal_sf(x)` — survival function 1−Φ(x); numerically stable for large x
- `log_gamma(x)` — ln(Γ(x)); Lanczos approximation (g=7, n=9), relative error < 1×10⁻¹³
- `incomplete_beta(x, a, b)` — regularised incomplete beta I_x(a,b); Lentz continued fractions, up to 200 iterations, relative error < 1×10⁻¹⁰
- `incomplete_gamma_lower(a, x)` — regularised lower incomplete gamma P(a,x); series + CF
- `t_cdf(t, df)` — CDF of Student's t-distribution via incomplete beta
- `p_value_t_two(t, df)` — two-tailed p-value: `2·P(T > |t|)`
- `chi2_cdf(x, df)` — CDF of chi-squared distribution (reserved for future Kruskal-Wallis)
- `chi2_sf(x, df)` — survival function of chi-squared

Built-in Rust unit tests (`#[cfg(test)]`) verify critical values against known references:
`erf(1) ≈ 0.8427`, `normal_cdf(1.96) ≈ 0.975`, `t_cdf(2.228, 10) ≈ 0.975`,
`chi2_cdf(3.841, 1) ≈ 0.95`.

**`liphia_compiler` — 0.9.1** (liphia_compiler/src/type_checker.rs)

- Type checker updated with declarations for all new `math` and `stats` functions
- New functions are statically type-checked before VM execution: argument count is validated at compile time; return types are propagated through expressions

---

## [0.9.0] — Engine 0.9.0

### Added

**Language**
- Indentation-based block syntax (Indent / Dedent tokens)
- Typed variable declarations (`name: type = value`)
- Inferred declarations (`var name = value`)
- Constants (`const name = value`)
- Optional types (`T?`, e.g. `int?`, `str?`)
- Conditionals: `if`, `elif`, `else`
- Loops: `while`, `for from/to/step`, `break`, `continue`
- Functions: `fn`, `return`, full recursion support
- Async functions: `async fn`, `await`
- Concurrency: `spawn` (fire-and-forget coroutine launch)
- Lists: literals, indexing (`x[0]`, `x[-1]`), `append`, `pop`
- Enums: `enum` declaration and variant access (`Enum.Variant`)
- Logical operators: `and`, `or`, `not` (also `&&`, `||`, `!`)
- String concatenation via `+`

**Compiler**
- Lexer with structured error reporting (line + column)
- Parser producing a typed AST
- Static type checker with forward-reference support for functions and enums
- Bytecode compiler with two-pass function resolution
- `async fn` compiled to suspendable coroutine entry points
- `spawn` resolved to `Opcode::Spawn` at compile time
- `await` resolved to `Opcode::Suspend` with polling semantics
- `break` and `continue` with correct patch-back to loop boundaries
- `import "file.lph"` — source-level file inclusion with cycle detection
- `import from "module"` — stdlib module resolution

**Virtual Machine**
- Stack-based bytecode VM with call frames and local variable slots
- Cooperative event-loop scheduler: round-robin task queue (`VecDeque<Task>`)
- `Opcode::Suspend` — polling model for async native calls
- `Opcode::Spawn` — creates independent coroutine tasks
- Quantum-based scheduling (256 instructions per task per tick)
- Values: `Int`, `Float`, `Bool`, `Str`, `List`, `EnumVariant`, `Null`
- `Rc<RefCell<Vec<Value>>>` for shared mutable lists
- Division by zero runtime error
- Stack underflow detection with context label

**CLI**
- `liphia <file.lph>` — compile and run
- `liphia --repl` — interactive shell
- `liphia init` — initialize `liphia.toml` in current directory
- `liphia install <module>` — download module from registry to `liphia_modules/`
- `liphia install` — install all dependencies from `liphia.toml`
- `liphia install --list` — list available stdlib modules
- `--no-cache` flag to force recompilation
- Bytecode cache: `.lbc` files in `liphia_cache/` alongside source
- FNV-based source hash for cache invalidation
- Binary cache format with magic bytes, version, and hash validation

**Core native functions** (always available, no import required)
- `len`, `to_int`, `to_float`, `to_str`
- `trim`, `upper`, `lower`
- `contains`, `starts_with`, `ends_with`
- `replace`, `split`
- `append`, `pop`

**Standard library** (install via `liphia install <name>`)
- `ai` — `sigmoid`, `relu`, `softmax`, `argmax`, `dot`, `norm`, `matrix_new`, `matrix_mul`, `matrix_add`
- `math` — `sqrt`, `pow`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `pi`, `e`, `log`, `log10`, `sin`, `cos`, `tan`
- `stats` — `sum`, `mean`, `median`, `variance`, `stdev`, `min_list`, `max_list`, `count`
- `fs` — `read_file`, `write_file`, `append_file`, `file_exists`
- `http` — `http_listen`, `http_accept`, `http_method`, `http_path`, `http_body`, `http_respond`, `http_respond_json`, `http_get`, `http_post`, `http_put`, `http_patch`, `http_delete`, `http_status`, `http_header`, `http_query`
- `ws` — `ws_listen`, `ws_accept`, `ws_clients`, `ws_send`, `ws_recv`, `ws_broadcast`, `ws_close`
- `net` — `tcp_connect`, `tcp_send`, `tcp_recv`, `tcp_close`, `udp_send`
- `json` — `json_encode`, `json_decode`, `json_get`, `json_has`
- `db` — SQLite: `db_open`, `db_exec`, `db_query`, `db_query_rows`, `db_close`, `db_last_id`, `db_begin`, `db_commit`, `db_rollback`, `db_error`, `db_tables`, `db_columns`; PostgreSQL: `pg_connect`, `pg_exec`, `pg_query`, `pg_query_rows`, `pg_close`, `pg_last_id`, `pg_begin`, `pg_commit`, `pg_rollback`, `pg_error`

**Tooling**
- VS Code extension with syntax highlighting for `.lph` files
- Workspace layout: `src/` as unified Cargo workspace covering `liphia_engine` and `stdlib/native`