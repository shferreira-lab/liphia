// java/Bench.java
//
// Java benchmark workloads, same algorithms as liphia/*.lph.
// Usage: javac Bench.java && java Bench <workload>
// Prints one checksum line; the runner times the whole process externally.

import java.util.ArrayList;
import java.util.HashMap;
import java.util.Map;

public class Bench {

    static long fib(int n) {
        if (n < 2) return n;
        return fib(n - 1) + fib(n - 2);
    }

    static long nestedSum(int n) {
        long total = 0;
        for (long i = 0; i < n; i++) {
            for (long j = 0; j < n; j++) {
                total = total + i * j;
            }
        }
        return total;
    }

    static long mandelbrot(int size, int maxIter) {
        long inside = 0;
        for (int y = 0; y < size; y++) {
            double ci = (double) y * 2.0 / (double) size - 1.0;
            for (int x = 0; x < size; x++) {
                double cr = (double) x * 3.0 / (double) size - 2.0;
                double zr = 0.0;
                double zi = 0.0;
                int k = 0;
                while (k < maxIter && zr * zr + zi * zi <= 4.0) {
                    double t = zr * zr - zi * zi + cr;
                    zi = 2.0 * zr * zi + ci;
                    zr = t;
                    k++;
                }
                if (k == maxIter) inside++;
            }
        }
        return inside;
    }

    // ArrayList<Boolean> instead of boolean[] so the list work stays
    // comparable to the dynamic lists of the other languages.
    static long countPrimes(int limit) {
        ArrayList<Boolean> isPrime = new ArrayList<>();
        for (int i = 0; i < limit; i++) isPrime.add(true);
        isPrime.set(0, false);
        isPrime.set(1, false);
        for (long p = 2; p * p < limit; p++) {
            if (isPrime.get((int) p)) {
                for (long m = p * p; m < limit; m += p) isPrime.set((int) m, false);
            }
        }
        long count = 0;
        for (int i = 0; i < limit; i++) if (isPrime.get(i)) count++;
        return count;
    }

    static long bubbleSortChecksum(int n) {
        ArrayList<Long> items = new ArrayList<>();
        for (int i = 0; i < n; i++) items.add((long) (n - i));
        for (int end = n - 1; end > 0; end--) {
            for (int j = 0; j < end; j++) {
                if (items.get(j) > items.get(j + 1)) {
                    Long t = items.get(j);
                    items.set(j, items.get(j + 1));
                    items.set(j + 1, t);
                }
            }
        }
        long check = 0;
        for (int i = 0; i < n; i++) check = check + items.get(i) * (i + 1);
        return check;
    }

    static long wordCount(int keys, int rounds) {
        HashMap<String, Long> counts = new HashMap<>();
        for (int r = 0; r < rounds; r++) {
            for (int k = 0; k < keys; k++) {
                String word = "w" + k;
                counts.merge(word, 1L, Long::sum);
            }
        }
        long total = 0;
        for (Map.Entry<String, Long> e : counts.entrySet()) {
            total = total + e.getValue() * e.getKey().length();
        }
        return total;
    }

    public static void main(String[] args) {
        String w = args[0];
        String out;
        switch (w) {
            case "startup":    out = "startup"; break;
            case "fib":        out = "fib " + fib(32); break;
            case "loops":      out = "loops " + nestedSum(3000); break;
            case "mandelbrot": out = "mandelbrot " + mandelbrot(300, 100); break;
            case "sieve":      out = "sieve " + countPrimes(2000000); break;
            case "sort":       out = "sort " + bubbleSortChecksum(2500); break;
            case "map":        out = "map " + wordCount(1000, 200); break;
            default: throw new IllegalArgumentException("unknown workload: " + w);
        }
        System.out.println(out);
    }
}
