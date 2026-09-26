# Liphia Benchmark

Six workloads, each implemented with the same algorithm in Liphia, Python,
Node.js and Java. Every program prints a checksum; the runner times the whole
process externally (best of 3) and rejects any result whose checksum differs.
Suite, runners and method: [`src/tests/bench/`](./src/tests/bench/README.md).

## Results — 2026-09-26, Liphia 2.0.0

Windows, x86_64. Time in milliseconds, whole process (start + compile + run),
best of 3 runs.

| Workload | What it stresses | Liphia | Python | Node.js | Java |
|---|---|---:|---:|---:|---:|
| startup | process start, no work | **13** | 33 | 57 | 109 |
| fib | recursive calls (`fib(32)`) | 1,760 | 409 | 82 | 125 |
| loops | 9M loop iterations, int math | 3,251 | 774 | 71 | 122 |
| mandelbrot | float math | 3,188 | 415 | 69 | 130 |
| sieve | list append and indexing | 2,895 | 601 | 142 | 199 |
| sort | list reads and writes | 3,048 | 458 | 61 | 152 |
| map | string keys in a map | 2,444 | 87 | 74 | 167 |
| **total** (without startup) | | **16,586** | 2,744 | 499 | 895 |

Versions: Python 3, Node.js (V8), Java 25.0.1 (HotSpot).

### Reading the results

- **Startup is Liphia's strength**: 13 ms from process start to output,
  2–8× faster than the other runtimes.
- **Execution speed is not, yet.** Liphia is about 4–8× slower than Python on
  the compute workloads and 28× slower on `map`. Node.js and Java compile hot
  code to machine code (JIT) and belong to another category; the fair
  reference for a bytecode interpreter is Python.
- 2.0.0 is the correctness release (semantics, packages, installers); this
  table is the baseline the performance work below is measured against.

## Performance roadmap

Ordered by effort versus gain. The first two are measured already on a Linux
machine (same suite; absolute numbers differ from the table above, ratios
carry over):

| Step | Change | Measured / expected effect |
|---|---|---|
| 1 | Release profile `opt-level = 3` instead of `"z"` (optimize for speed, not size) | **~2× faster** everywhere; binary 1.5 → 1.9 MB |
| 2 | VM loop borrows each instruction instead of cloning it | **~1.3× on top of step 1**; together ~2.8× |
| 3 | `map` backed by a hash table instead of a list of pairs | `map` from O(n) to O(1) lookups; closes most of the 28× gap |
| 4 | LBC v5: constant pool, globals by slot, natives by index | removes string hashing and allocation from calls and global access |
| 5 | Fused instructions for the hottest patterns (compare + jump, `i = i + 1`) | fewer dispatches per loop iteration |
| 6 | Register-based VM or a JIT (via LiphiaB / Cranelift) | the long-term path toward Node.js/Java territory |

## History

Earlier benchmarks (March–May 2026, engines 0.5–0.9) timed very small
programs — `fib(25)`–`fib(30)` and a few vector operations — externally.
Those programs finish in a few milliseconds in every runtime, so the
measurement was dominated by process startup, where Liphia leads. They showed
Liphia ahead of Node.js, Java and Python (e.g. 9 ms vs 58 / 124 / 159 ms on
31/03/2026), but that reflected startup cost, not execution speed. The suite
above separates the two.

###  – 31/03/2026 - Tests Overview    (Old engine with modules-stdlib)
- **Recursive Fibonacci** (`fib(30)`) – measures performance on heavy recursion.  
- **AI Primitives** (`sigmoid`, `relu`, `dot`, `norm`) – basic mathematical functions used in AI.  
- **Softmax + Argmax** – computes probabilities and selects the action with the highest score.

### Benchmark Results (best of 3 runs, external timing)

| Language      | Time (ms) | Notes                       |
|---------------|-----------|-----------------------------|
| Liphia (VM)   | 9.14      | best of 3 (external)       |
| Node.js (V8)  | 57.85     | best of 3 (external)       |
| Java (JVM)    | 124.09    | best of 3 (external)       |
| Python 3      | 158.88    | best of 3 (external)       |


### May 05, 2026    - Liphia 0.9 with stdlib, lastest updates. 
Language    Time_ms    Note                 
---------    -------- ----                 
Liphia (VM)     91,69 best of 3 (external)
Node.js (V8)   125,72 best of 3 (external)
Java (JVM)     235,15 best of  3 (external)
Python 3       405,76 best of  3 (external)

All timings were measured externally using Stopwatch.
Method: best of 3 runs.