# Liphia examples

Run any example with `liphia <file>.lph`.

The files at the top level use only core natives, which are compiled into
the binary and need no import or install:

| File | Shows |
|------|-------|
| `01_basics.lph` | variables, const, enum, functions, if/elif/else, for, while |
| `02_collections.lph` | strings, lists, maps, conversions |
| `03_errors.lph` | runtime errors, try/catch, no implicit int/float mixing |
| `04_async.lph` | async fn, spawn, round-robin tasks |
| `05_math_random.lph` | scalar math, sum/mean/min_list/max_list, seeded random |
| `06_json_files.lph` | files, read_json/write_json, JSON Lines |
| `07_http_server.lph` | HTTP server with await on http_accept |
| `08_http_client.lph` | HTTP client against the server from 07 |
| `09_ws_broadcast.lph` | WebSocket relay server |

The files in `packages/` need the package installed first
(`liphia install <name>` in the folder you run them from):

| File | Package |
|------|---------|
| `packages/10_wire_api.lph` | wire — JSON API with response helpers |
| `packages/11_num.lph` | num — vectors, matrices, descriptive statistics |
| `packages/12_stats.lph` | stats — hypothesis tests, compare_groups |
| `packages/13_learn.lph` | learn — logistic regression trained with SGD |
| `packages/14_db_sqlite.lph` | db — SQLite in memory |
