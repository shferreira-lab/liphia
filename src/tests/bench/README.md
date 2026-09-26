# Liphia benchmarks

Six workloads, each written with the same algorithm in Liphia, Python,
Node.js and Java. Every program prints one checksum line; the runner times
the whole process from outside, keeps the best of N runs, and flags any
language whose checksum differs (a wrong answer makes the time meaningless).

| Workload | What it stresses |
|----------|------------------|
| `startup` | process start + compile + one `print`: the fixed cost of each runtime |
| `fib` | recursive `fib(32)`: function calls, int arithmetic, branches |
| `loops` | 3000 × 3000 nested `while` loops: the interpreter's instruction dispatch |
| `mandelbrot` | 300 × 300 grid, 100 iterations: float arithmetic |
| `sieve` | primes below 2,000,000: list append, index read/write |
| `sort` | bubble sort of 2,500 ints: list reads/writes and comparisons |
| `map` | 1,000 string keys × 200 rounds: map lookup and update, string building |

Sizes are chosen so the slow runtimes take seconds per workload: with
tiny workloads the measurement is dominated by process startup (tens of
milliseconds for Node.js and the JVM, a few for Liphia), which says nothing
about execution speed. `startup` is measured separately for that reason and
left out of the total.

## Running

```powershell
cd bench
powershell -ExecutionPolicy Bypass -File .\benchmark.ps1
powershell -ExecutionPolicy Bypass -File .\benchmark.ps1 -Runs 5 -Only fib,sort
```

```bash
cd bench
sh benchmark.sh
RUNS=5 ONLY="fib sort" sh benchmark.sh
```

Liphia is taken from `PATH` (`liphia`); point `-Liphia` (PowerShell) or
`LIPHIA=` (shell) at another executable to compare builds. Python, Node.js
and Java are used when found (Java needs a JDK for `javac`). Each run saves
the table as `results-<date>.md`.

## Reading the results

- Times are whole-process, in milliseconds: start, compile (Liphia uses its
  bytecode cache after the warm-up run; Java is compiled once with `javac`
  before timing), run, exit.
- Node.js and Java compile hot code to machine code (JIT); Python and Liphia
  interpret bytecode. The fair comparison for Liphia is Python.
- Compare runs from the same machine only.
