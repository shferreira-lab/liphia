#!/bin/bash
# benchmark.sh — roda fib(30) em cada linguagem e exibe tabela comparativa
# Uso: bash benchmark.sh

LIPHIA_CLI="../motor_liphia/target/debug/liphia_cli"

echo ""
echo "========================================"
echo "  Benchmark fib(30) — Motor Liphia 0.5 "
echo "========================================"
echo ""

ms() {
    # recebe comando e retorna tempo em ms
    local start end
    start=$(date +%s%N)
    "$@" > /dev/null 2>&1
    end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

# --- Liphia ---
if [ -f "$LIPHIA_CLI" ]; then
    echo "Liphia..."
    OUT=$("$LIPHIA_CLI" bench_fib.lph 2>&1)
    T=$(ms "$LIPHIA_CLI" bench_fib.lph)
    echo "  $OUT"
    echo "  tempo total (startup+exec): ${T} ms"
    LIPHIA_T=$T
else
    echo "[ERRO] liphia_cli nao encontrado em $LIPHIA_CLI"
    echo "       Ajuste o caminho LIPHIA_CLI no script se necessario."
fi

# --- Python ---
if command -v python3 &>/dev/null; then
    echo "Python..."
    python3 bench_fib.py
    T=$(ms python3 bench_fib.py)
    echo "  tempo total (startup+exec): ${T} ms"
fi

# --- TypeScript ---
if command -v npx &>/dev/null; then
    echo "TypeScript (ts-node)..."
    npx ts-node bench_fib.ts 2>/dev/null
    T=$(ms npx ts-node bench_fib.ts)
    echo "  tempo total (startup+exec): ${T} ms"
fi

# --- Java ---
if command -v javac &>/dev/null; then
    echo "Java..."
    javac BenchFib.java 2>/dev/null
    java BenchFib
    T=$(ms java BenchFib)
    echo "  tempo total (startup+exec): ${T} ms"
fi

echo ""
echo "Obs: 'tempo interno' = so execucao da funcao"
echo "     'tempo total'   = startup da linguagem + execucao"
echo ""
