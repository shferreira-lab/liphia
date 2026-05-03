# Liphia Benchmark – 31/03/2026

## Tests Overview
- **Recursive Fibonacci** (`fib(30)`) – measures performance on heavy recursion.  
- **AI Primitives** (`sigmoid`, `relu`, `dot`, `norm`) – basic mathematical functions used in AI.  
- **Softmax + Argmax** – computes probabilities and selects the action with the highest score.

## Benchmark Results (best of 3 runs, external timing)

| Language      | Time (ms) | Notes                       |
|---------------|-----------|-----------------------------|
| Liphia (VM)   | 9.14      | best of 3 (external)       |
| Node.js (V8)  | 57.85     | best of 3 (external)       |
| Java (JVM)    | 124.09    | best of 3 (external)       |
| Python 3      | 158.88    | best of 3 (external)       |

> **Observation:** Liphia’s VM performs extremely well, significantly faster than Node.js, Java, and Python for these tasks.




May 05, 2026    - Liphia 0.9 with stdlib, lastest updates. 
Language    Time_ms    Note                 
---------    -------- ----                 
Liphia (VM)     91,69 best of 3 (external)
Node.js (V8)   125,72 best of 3 (external)
Java (JVM)     235,15 best of  3 (external)
Python 3       405,76 best of  3 (external)