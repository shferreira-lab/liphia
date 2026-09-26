#!/bin/sh
# benchmark.sh
#
# Runs every workload in every available language, times each whole process
# externally (best of N runs), checks that all languages print the same
# checksum, and prints a table (also saved as results-<date>.md).
#
#   sh benchmark.sh              # best of 3
#   RUNS=5 sh benchmark.sh
#   ONLY="fib sort" sh benchmark.sh
#   LIPHIA=/path/to/liphia sh benchmark.sh
#
# Timing uses `date +%s%N`, available on Linux; on macOS install GNU
# coreutils (`brew install coreutils`) so `gdate` can be used instead.

set -eu
cd "$(dirname "$0")"

RUNS="${RUNS:-3}"
LIPHIA="${LIPHIA:-liphia}"
WORKLOADS="${ONLY:-startup fib loops mandelbrot sieve sort map}"

if date +%s%N | grep -q N; then
    command -v gdate >/dev/null 2>&1 || { echo "need GNU date (gdate) for ms timing"; exit 1; }
    now() { gdate +%s%N; }
else
    now() { date +%s%N; }
fi

file_for() {
    case "$1" in
        startup) echo 00_startup.lph ;;   fib) echo 01_fib.lph ;;
        loops) echo 02_loops.lph ;;       mandelbrot) echo 03_mandelbrot.lph ;;
        sieve) echo 04_sieve.lph ;;       sort) echo 05_sort.lph ;;
        map) echo 06_map.lph ;;
    esac
}

LANGS=""
command -v "$LIPHIA" >/dev/null 2>&1 && LANGS="$LANGS Liphia"
PYTHON=$(command -v python3 || command -v python || true)
[ -n "$PYTHON" ] && LANGS="$LANGS Python"
command -v node >/dev/null 2>&1 && LANGS="$LANGS Node.js"
if command -v javac >/dev/null 2>&1 && command -v java >/dev/null 2>&1; then
    javac -d java java/Bench.java && LANGS="$LANGS Java"
fi
[ -n "$LANGS" ] || { echo "no language found"; exit 1; }

run() {
    case "$1" in
        Liphia)  "$LIPHIA" "liphia/$(file_for "$2")" ;;
        Python)  "$PYTHON" python/bench.py "$2" ;;
        Node.js) node node/bench.js "$2" ;;
        Java)    java -cp java Bench "$2" ;;
    esac
}

echo ""
echo "Liphia benchmark - best of $RUNS runs, external timing"
echo "Languages:$LANGS"
echo ""

for lang in $LANGS; do run "$lang" startup >/dev/null 2>&1; done

DATE=$(date +%Y-%m-%d)
MD="results-$DATE.md"
{
    echo "# Liphia benchmark - $DATE"
    echo ""
    echo "Best of $RUNS runs, whole process timed externally (ms). Machine: $(uname -sm)."
    echo ""
    printf "| Workload |"; for lang in $LANGS; do printf " %s |" "$lang"; done; echo ""
    printf "|---|"; for lang in $LANGS; do printf -- "---:|"; done; echo ""
} > "$MD"

TOTALS=""
for w in $WORKLOADS; do
    ref=""
    line="| $w |"
    for lang in $LANGS; do
        best=""
        out=""
        i=0
        while [ "$i" -lt "$RUNS" ]; do
            s=$(now); out=$(run "$lang" "$w" 2>/dev/null); e=$(now)
            ms=$(( (e - s) / 1000000 ))
            if [ -z "$best" ] || [ "$ms" -lt "$best" ]; then best=$ms; fi
            i=$((i + 1))
        done
        [ -z "$ref" ] && ref="$out"
        mark=""
        [ "$out" != "$ref" ] && mark=" !"
        printf "  %-11s %-8s %8s ms   %s%s\n" "$w" "$lang" "$best" "$out" "$mark"
        line="$line $best$mark |"
        if [ "$w" != "startup" ]; then
            prev=$(echo "$TOTALS" | tr ' ' '\n' | grep "^$lang=" | cut -d= -f2 || true)
            TOTALS=$(echo "$TOTALS" | tr ' ' '\n' | grep -v "^$lang=" | tr '\n' ' ')
            TOTALS="$TOTALS $lang=$(( ${prev:-0} + best ))"
        fi
    done
    echo "$line" >> "$MD"
done

line="| TOTAL (no startup) |"
echo ""
echo "TOTAL (no startup):"
for lang in $LANGS; do
    t=$(echo "$TOTALS" | tr ' ' '\n' | grep "^$lang=" | cut -d= -f2)
    printf "  %-8s %8s ms\n" "$lang" "$t"
    line="$line $t |"
done
echo "$line" >> "$MD"
echo ""
echo "Times in ms, whole process (start + compile + run). Rows marked '!' printed a different checksum."
echo "Saved $MD"