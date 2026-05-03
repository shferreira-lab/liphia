import time
import math

# Fibonacci recursivo
def fib_rec(n):
    if n <= 1: return n
    return fib_rec(n-1) + fib_rec(n-2)

# Fibonacci iterativo
def fib_iter(n):
    a,b = 0,1
    for _ in range(n):
        a,b = b,a+b
    return a

# Dot, norm, softmax
v1 = [1.0,2.0,3.0,4.0,5.0]
v2 = [5.0,4.0,3.0,2.0,1.0]

start = time.perf_counter()
r1 = fib_rec(25)
t1 = (time.perf_counter() - start) * 1000

start = time.perf_counter()
r2 = fib_iter(35)
t2 = (time.perf_counter() - start) * 1000

start = time.perf_counter()
dot_val = sum(a*b for a,b in zip(v1,v2))
norm_val = math.sqrt(sum(a**2 for a in v1))
exp_v = [math.exp(x) for x in v1]
s = sum(exp_v)
softmax = [x/s for x in exp_v]
t3 = (time.perf_counter() - start) * 1000

print("fib_rec(25) =", r1, "| tempo =", round(t1,2),"ms")
print("fib_iter(35) =", r2, "| tempo =", round(t2,2),"ms")
print("dot(v1,v2) =", dot_val)
print("norm(v1) =", norm_val)
print("softmax(v1) =", softmax, "| tempo =", round(t3,2),"ms")