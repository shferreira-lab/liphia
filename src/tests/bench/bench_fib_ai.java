// BenchFibAI.java
//
// Java — Benchmark
//
// Measures:
//   - Recursive Fibonacci fib(25)
//   - Iterative Fibonacci fib(35)
//   - AI vector operations: dot, norm, softmax (pure Java, no external libs)

import java.util.Arrays;

public class BenchFibAI {

    static int fibRec(int n) {
        if (n <= 1) return n;
        return fibRec(n - 1) + fibRec(n - 2);
    }

    static int fibIter(int n) {
        int a = 0, b = 1;
        for (int i = 0; i < n; i++) {
            int temp = a;
            a = b;
            b = temp + b;
        }
        return a;
    }

    static double dot(double[] v1, double[] v2) {
        double sum = 0;
        for (int i = 0; i < v1.length; i++) sum += v1[i] * v2[i];
        return sum;
    }

    static double norm(double[] v) {
        double sum = 0;
        for (double x : v) sum += x * x;
        return Math.sqrt(sum);
    }

    static double[] softmax(double[] v) {
        double[] exp = new double[v.length];
        double sum = 0;
        for (int i = 0; i < v.length; i++) { exp[i] = Math.exp(v[i]); sum += exp[i]; }
        for (int i = 0; i < v.length; i++) exp[i] /= sum;
        return exp;
    }

    public static void main(String[] args) {
        double[] v1 = {1, 2, 3, 4, 5};
        double[] v2 = {5, 4, 3, 2, 1};

        long start, elapsed;

        start   = System.nanoTime();
        int r1  = fibRec(25);
        elapsed = System.nanoTime() - start;
        System.out.printf("fib_rec(25)  = %d | time = %.4f ms%n", r1, elapsed / 1e6);

        start   = System.nanoTime();
        int r2  = fibIter(35);
        elapsed = System.nanoTime() - start;
        System.out.printf("fib_iter(35) = %d | time = %.4f ms%n", r2, elapsed / 1e6);

        start = System.nanoTime();
        double dotVal  = dot(v1, v2);
        double normVal = norm(v1);
        double[] sm    = softmax(v1);
        elapsed = System.nanoTime() - start;
        System.out.printf("dot(v1,v2)   = %.1f%n",   dotVal);
        System.out.printf("norm(v1)     = %.15f%n",  normVal);
        System.out.printf("softmax(v1)  = %s | time = %.4f ms%n", Arrays.toString(sm), elapsed / 1e6);
    }
}