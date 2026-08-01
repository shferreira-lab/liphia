# Liphia Standard Library — Function Reference

This is the single, versioned reference for every stdlib module: what's
**native** (compiled into every binary, callable regardless of import) versus
what's **composed** (pure `.lph`, only reaches your project after
`liphia install <module>`).

See the [stdlib README](./README.md) for install instructions and module
versions. This file documents *what each module provides*, not how to fetch it.

---

## Table of Contents

- [ai — 1.1.0](#ai--110)
- [math — 1.1.0](#math--110)
- [stats — 1.1.0](#stats--110)
- [http — 1.2.0](#http--120)
- [json — 1.2.0](#json--120)
- [fs — 1.1.0](#fs--110)
- [net — 1.1.0](#net--110)
- [ws — 1.1.0](#ws--110)
- [db — 1.1.0](#db--110)

---

## `ai` — 1.1.0

AI and neural network primitives. Matrices are flat lists in row-major order.

### Native

| Function | Returns | Description |
|---|---|---|
| `sigmoid(x)`, `relu(x)`, `leaky_relu(x, alpha)`, `tanh_act(x)`, `elu(x, alpha)`, `gelu(x)`, `swish(x)` | `float` | Activation functions |
| `dot(a, b)`, `norm(v)`, `vec_add(a, b)`, `vec_sub(a, b)`, `vec_mul(a, b)`, `vec_scale(v, s)`, `vec_sum(v)` | varies | Vector operations |
| `softmax(v)`, `argmax(v)` | `list` / `int` | Classification helpers |
| `matrix_new(rows, cols, fill)`, `matrix_mul(a, b, rows, inner, cols)`, `matrix_add(a, b)`, `transpose(m, rows, cols)` | `list` | Flat row-major matrix ops |
| `normalize(v)`, `standardize(v)`, `clip(v, min, max)`, `linspace(start, end, n)`, `arange(start, end, step)` | `list` | Data preprocessing |
| `mse(pred, target)`, `mae(pred, target)`, `cross_entropy(pred, target)`, `binary_cross_entropy(pred, target)` | `float` | Loss functions |
| `seed(n)`, `rand_uniform(n, low, high)`, `rand_normal(n, mean, std)`, `rand_int(low, high)`, `shuffle(v)` | varies | Random |
| `gradient_clip(grads, max_norm)`, `sgd_update(weights, grads, lr)`, `adam_update(weights, grads, m, v, t, lr, beta1, beta2, eps)` | `list` | Gradients / optimization |
| `accuracy(pred, target)`, `precision(pred, target)`, `recall(pred, target)`, `f1_score(pred, target)` | `float` | Classification metrics — threshold 0.5 |
| `cosine_similarity(a, b)`, `euclidean_dist(a, b)`, `manhattan_dist(a, b)` | `float` | Distance functions |

### Composed (`composed/eval.lph`, via `liphia install ai`)

| Function | Returns | Description |
|---|---|---|
| `classification_report(pred, target)` | `map` | Bundles accuracy/precision/recall/f1_score into one map |
| `is_better(scores_a, scores_b)` | `map` | Compares two score sets (e.g. k-fold runs): picks paired-t or Mann-Whitney automatically (via Shapiro-Wilk normality check), returns `{mean_a, mean_b, test, p_value, significant, b_is_better}`. Calls `stats` natives directly — no `import from "stats"` needed |

---

## `math` — 1.1.0

| Category | Functions |
|---|---|
| Basic | `sqrt`, `pow`, `abs`, `floor`, `ceil`, `round`, `min`, `max` |
| Constants | `pi()`, `e()` |
| Logarithm / exponential | `log`, `log10`, `log2`, `log_base`, `exp` |
| Trigonometry | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |
| Hyperbolic | `sinh`, `cosh`, `tanh` |
| Number theory | `factorial`, `gcd`, `lcm` |
| Geometry | `hypot`, `deg_to_rad`, `rad_to_deg` |
| Utilities | `sign`, `clamp`, `is_nan`, `is_inf` |

### Composed (`summary.lph`, via `liphia install math`)

Pre-existing composed helpers built on the natives above — see
`modules/math/summary.lph` directly for the current function list.

---

## `stats` — 1.1.0

### Native

| Category | Functions |
|---|---|
| Descriptive | `sum`, `mean`, `median`, `min_list`, `max_list`, `count`, `mode`, `range_stat` |
| Variance / spread | `variance`, `stdev`, `variance_sample`, `stdev_sample`, `percentile(data, p)`, `iqr` |
| Normalisation | `zscore`, `covariance` |
| Correlation | `pearson_r`, `spearman_r`, `kendall_tau` |
| Test statistics | `t_stat_independent`, `t_degrees_of_freedom`, `t_stat_paired`, `mann_whitney_u`, `wilcoxon_w` |
| Normality | `shapiro_wilk_w` |
| p-values | `p_value_t_ind`, `p_value_t_paired`, `p_value_normal`, `p_value_mann_whitney` (requires `min(n1,n2) > 10`) |

### Composed (`composed/report.lph`, via `liphia install stats`)

| Function | Returns | Description |
|---|---|---|
| `describe(data)` | `map` | `{mean, median, stdev, min, max, q1, q3, iqr}` in one call |
| `compare_groups(a, b)` | `map` | Automates the normal-vs-nonparametric decision guide: Shapiro-Wilk on both groups, then Welch's t-test or Mann-Whitney. When both groups fail normality and `min(len(a), len(b)) <= 10`, returns `p_value: -1.0` and `test: "mann_whitney_u_table_needed"` — signals to fall back to `mann_whitney_u()` with a printed critical-value table |

---

## `http` — 1.2.0

Native CORS on every response (`Access-Control-Allow-*` headers).

### Native

| Function | Returns | Description |
|---|---|---|
| `http_listen(port)` | `bool` | Bind server, spawn accept thread |
| `http_accept()` | `bool` | True if a request is ready — poll via `await` in an `async fn` |
| `http_method()`, `http_path()`, `http_query()`, `http_body()` | `str` | Current request accessors |
| `http_header(name)` | `str` | Request header value (lowercase key) |
| `http_respond(status, body)` | `bool` | text/plain response |
| `http_respond_json(status, body)` | `bool` | application/json response |
| `http_get(url)`, `http_post(url, body)`, `http_put(url, body)`, `http_patch(url, body)`, `http_delete(url)` | `str` | Client requests |
| `http_status()` | `int` | Last client response status code |

### Composed (`composed/respond.lph`, via `liphia install http`)

| Function | Returns | Description |
|---|---|---|
| `ok_json(data)` | `bool` | `http_respond_json(200, json_encode(data))` |
| `created_json(data)` | `bool` | 201 |
| `no_content()` | `bool` | 204, empty body |
| `error_json(status, message)` | `bool` | `{status, {"error": message}}` |
| `not_found_json(message)` | `bool` | 404 |
| `bad_request_json(message)` | `bool` | 400 |

> A server `route()` (or similar) called via `await` inside an `async fn`
> should always return a non-`void` type (e.g. `bool`, as the helpers above
> do) rather than `void`/no explicit return — see the Engine changelog for
> why this matters for the async scheduler.

---

## `json` — 1.2.0

| Function | Returns | Description |
|---|---|---|
| `json_encode(value)` | `str` | Serializes any Liphia value |
| `json_decode(text)` | `map` / `list` / scalar | Objects → `map`, arrays → `list` (breaking change from the old flat-list format) |
| `json_get(text, key)` | `str` | String representation of a key's value, without decoding first |
| `json_has(text, key)` | `bool` | Key presence check |

No composed layer of its own — `json` is instead composed *into* other
modules (`http`, `fs`, `net`, `ws` all layer JSON helpers on top of their own
natives + `json`'s).

---

## `fs` — 1.1.0

### Native

| Function | Returns | Description |
|---|---|---|
| `read_file(path)` | `str` | Errors if the file can't be opened |
| `write_file(path, content)` | `bool` | Overwrites, creates if missing |
| `append_file(path, content)` | `bool` | Appends, creates if missing |
| `file_exists(path)` | `bool` | |

### Composed (`composed/json_io.lph`, via `liphia install fs`)

| Function | Returns | Description |
|---|---|---|
| `read_json(path)` | any | `json_decode(read_file(path))` |
| `write_json(path, data)` | `bool` | `write_file(path, json_encode(data))` |
| `append_json_line(path, data)` | `bool` | Appends one JSON-encoded line (JSONL) |

---

## `net` — 1.1.0

### Native

| Function | Returns | Description |
|---|---|---|
| `tcp_connect(host, port)` | `int` | Returns a connection handle |
| `tcp_send(handle, data)` | `bool` | |
| `tcp_recv(handle)` | `str` | Reads up to 4096 bytes available right now |
| `tcp_close(handle)` | `bool` | |
| `udp_send(host, port, data)` | `bool` | |

### Composed (`composed/stream.lph`, via `liphia install net`)

| Function | Returns | Description |
|---|---|---|
| `tcp_recv_all(handle)` | `str` | Loops `tcp_recv` until empty, accumulating a full multi-packet response |
| `tcp_send_json(handle, data)` | `bool` | `tcp_send(handle, json_encode(data))` |
| `tcp_recv_json(handle)` | any | `json_decode(tcp_recv_all(handle))` |

---

## `ws` — 1.1.0

### Native

| Function | Returns | Description |
|---|---|---|
| `ws_listen(port)` | `bool` | |
| `ws_accept()` | `int` | Next client handle, `0` if none |
| `ws_clients()` | `list` | Connected client handles |
| `ws_send(handle, msg)` | `bool` | |
| `ws_recv(handle)` | `str` | Next message from a client, `""` if none |
| `ws_broadcast(msg)` | `bool` | |
| `ws_close(handle)` | `bool` | |

> Planned, not yet implemented: `ws_on_connect`/`ws_on_message`/`ws_on_disconnect`
> event-callback registration — requires native-side dispatch into a Liphia
> function, not achievable as a composed `.lph` layer.

### Composed (`composed/json.lph`, via `liphia install ws`)

| Function | Returns | Description |
|---|---|---|
| `ws_send_json(handle, data)` | `bool` | `ws_send(handle, json_encode(data))` |
| `ws_broadcast_json(data)` | `bool` | `ws_broadcast(json_encode(data))` |

---

## `db` — 1.1.0

SQLite (embedded, bundled) and PostgreSQL (pure TCP wire protocol — trust and
cleartext password auth; no TLS/SCRAM yet).

### Native — SQLite

| Function | Returns | Description |
|---|---|---|
| `db_open(path)`, `db_open_memory()` | `int` | Connection handle |
| `db_close(handle)` | `bool` | |
| `db_exec(handle, sql)` | `int` | Rows affected |
| `db_query(handle, sql)` | `list` | Flat `["col", val, "col", val, ...]` across all rows |
| `db_query_rows(handle, sql)` | `list` | List of per-row flat lists |
| `db_last_id(handle)` | `int` | Last `INSERT` rowid |
| `db_begin(handle)`, `db_commit(handle)`, `db_rollback(handle)` | `bool` | Transactions |
| `db_error(handle)` | `str` | |
| `db_tables(handle)` | `list` | Table names |
| `db_columns(handle, table)` | `list` | `PRAGMA table_info` as a flat list |

### Native — PostgreSQL

| Function | Returns | Description |
|---|---|---|
| `pg_connect(host, port, user, pass, db)` | `int` | |
| `pg_exec(handle, sql)` | `int` | |
| `pg_query(handle, sql)`, `pg_query_rows(handle, sql)` | `list` | Same shape as the SQLite equivalents |
| `pg_last_id(handle)` | `int` | Use `RETURNING id` in your `INSERT` for reliability |
| `pg_begin`, `pg_commit`, `pg_rollback`, `pg_close` | `bool` | |
| `pg_error(handle)` | `str` | |

### Submodules — separately installable

```bash
liphia install db:sqlite
liphia install db:postgres
```
```lph
import { sqlite } from "db"
import { postgres } from "db"
```
Convenience helpers specific to one backend (e.g. `sqlite_table_exists`,
`pg_table_exists`) — see `modules/db/sqlite/` and `modules/db/postgres/`
directly for the current function list.