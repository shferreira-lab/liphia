# python/bench.py
#
# Python benchmark workloads, same algorithms as liphia/*.lph.
# Usage: python bench.py <workload>
# Prints one checksum line; the runner times the whole process externally.

import sys


def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def nested_sum(n):
    total = 0
    i = 0
    while i < n:
        j = 0
        while j < n:
            total = total + i * j
            j = j + 1
        i = i + 1
    return total


def mandelbrot(size, max_iter):
    inside = 0
    y = 0
    while y < size:
        ci = float(y) * 2.0 / float(size) - 1.0
        x = 0
        while x < size:
            cr = float(x) * 3.0 / float(size) - 2.0
            zr = 0.0
            zi = 0.0
            k = 0
            while k < max_iter and zr * zr + zi * zi <= 4.0:
                t = zr * zr - zi * zi + cr
                zi = 2.0 * zr * zi + ci
                zr = t
                k = k + 1
            if k == max_iter:
                inside = inside + 1
            x = x + 1
        y = y + 1
    return inside


def count_primes(limit):
    is_prime = []
    i = 0
    while i < limit:
        is_prime.append(True)
        i = i + 1
    is_prime[0] = False
    is_prime[1] = False
    p = 2
    while p * p < limit:
        if is_prime[p]:
            m = p * p
            while m < limit:
                is_prime[m] = False
                m = m + p
        p = p + 1
    count = 0
    i = 0
    while i < limit:
        if is_prime[i]:
            count = count + 1
        i = i + 1
    return count


def bubble_sort_checksum(n):
    items = []
    i = 0
    while i < n:
        items.append(n - i)
        i = i + 1
    end = n - 1
    while end > 0:
        j = 0
        while j < end:
            if items[j] > items[j + 1]:
                t = items[j]
                items[j] = items[j + 1]
                items[j + 1] = t
            j = j + 1
        end = end - 1
    check = 0
    i = 0
    while i < n:
        check = check + items[i] * (i + 1)
        i = i + 1
    return check


def word_count(keys, rounds):
    counts = {}
    r = 0
    while r < rounds:
        k = 0
        while k < keys:
            word = "w" + str(k)
            if word in counts:
                counts[word] = counts[word] + 1
            else:
                counts[word] = 1
            k = k + 1
        r = r + 1
    total = 0
    for name in counts:
        total = total + counts[name] * len(name)
    return total


WORKLOADS = {
    "startup": lambda: "startup",
    "fib": lambda: "fib %d" % fib(32),
    "loops": lambda: "loops %d" % nested_sum(3000),
    "mandelbrot": lambda: "mandelbrot %d" % mandelbrot(300, 100),
    "sieve": lambda: "sieve %d" % count_primes(2000000),
    "sort": lambda: "sort %d" % bubble_sort_checksum(2500),
    "map": lambda: "map %d" % word_count(1000, 200),
}

print(WORKLOADS[sys.argv[1]]())
