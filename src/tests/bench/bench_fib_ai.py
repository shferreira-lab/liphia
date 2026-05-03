# bench_fib_ai.py
#
# Python 3 — Benchmark
#
# Measures:
#   - Recursive Fibonacci fib(25)
#   - Iterative Fibonacci fib(35)
#   - AI vector operations: dot, norm, softmax (pure Python, no numpy)

import time
import math


def fib_rec(n):
    if n <= 1:
        return n
    return fib_rec(n - 1) + fib_rec(n - 2)


def fib_iter(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def dot(v1, v2):
    return sum(a * b for a, b in zip(v1, v2))


def norm(v):
    return math.sqrt(sum(x ** 2 for x in v))


def softmax(v):
    exp_v = [math.exp(x) for x in v]
    s = sum(exp_v)
    return [x / s for x in exp_v]


v1 = [1.0, 2.0, 3.0, 4.0, 5.0]
v2 = [5.0, 4.0, 3.0, 2.0, 1.0]

start = time.perf_counter()
r1 = fib_rec(25)
t1 = (time.perf_counter() - start) * 1000

start = time.perf_counter()
r2 = fib_iter(35)
t2 = (time.perf_counter() - start) * 1000

start = time.perf_counter()
dot_val  = dot(v1, v2)
norm_val = norm(v1)
sm       = softmax(v1)
t3 = (time.perf_counter() - start) * 1000

print(f"fib_rec(25)  = {r1} | time = {round(t1, 4)} ms")
print(f"fib_iter(35) = {r2} | time = {round(t2, 4)} ms")
print(f"dot(v1,v2)   = {dot_val}")
print(f"norm(v1)     = {norm_val}")
print(f"softmax(v1)  = {sm} | time = {round(t3, 4)} ms")