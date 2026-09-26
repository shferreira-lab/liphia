// node/bench.js
//
// Node.js benchmark workloads, same algorithms as liphia/*.lph.
// Usage: node bench.js <workload>
// Prints one checksum line; the runner times the whole process externally.
// Integer results stay below 2^53, so plain numbers are exact.

function fib(n) {
    if (n < 2) return n;
    return fib(n - 1) + fib(n - 2);
}

function nestedSum(n) {
    let total = 0;
    for (let i = 0; i < n; i++) {
        for (let j = 0; j < n; j++) {
            total = total + i * j;
        }
    }
    return total;
}

function mandelbrot(size, maxIter) {
    let inside = 0;
    for (let y = 0; y < size; y++) {
        const ci = y * 2.0 / size - 1.0;
        for (let x = 0; x < size; x++) {
            const cr = x * 3.0 / size - 2.0;
            let zr = 0.0;
            let zi = 0.0;
            let k = 0;
            while (k < maxIter && zr * zr + zi * zi <= 4.0) {
                const t = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = t;
                k++;
            }
            if (k === maxIter) inside++;
        }
    }
    return inside;
}

function countPrimes(limit) {
    const isPrime = [];
    for (let i = 0; i < limit; i++) isPrime.push(true);
    isPrime[0] = false;
    isPrime[1] = false;
    for (let p = 2; p * p < limit; p++) {
        if (isPrime[p]) {
            for (let m = p * p; m < limit; m += p) isPrime[m] = false;
        }
    }
    let count = 0;
    for (let i = 0; i < limit; i++) if (isPrime[i]) count++;
    return count;
}

function bubbleSortChecksum(n) {
    const items = [];
    for (let i = 0; i < n; i++) items.push(n - i);
    for (let end = n - 1; end > 0; end--) {
        for (let j = 0; j < end; j++) {
            if (items[j] > items[j + 1]) {
                const t = items[j];
                items[j] = items[j + 1];
                items[j + 1] = t;
            }
        }
    }
    let check = 0;
    for (let i = 0; i < n; i++) check = check + items[i] * (i + 1);
    return check;
}

function wordCount(keys, rounds) {
    const counts = new Map();
    for (let r = 0; r < rounds; r++) {
        for (let k = 0; k < keys; k++) {
            const word = "w" + k;
            counts.set(word, (counts.get(word) || 0) + 1);
        }
    }
    let total = 0;
    for (const [name, count] of counts) total = total + count * name.length;
    return total;
}

const workloads = {
    startup: () => "startup",
    fib: () => `fib ${fib(32)}`,
    loops: () => `loops ${nestedSum(3000)}`,
    mandelbrot: () => `mandelbrot ${mandelbrot(300, 100)}`,
    sieve: () => `sieve ${countPrimes(2000000)}`,
    sort: () => `sort ${bubbleSortChecksum(2500)}`,
    map: () => `map ${wordCount(1000, 200)}`,
};

console.log(workloads[process.argv[2]]());
