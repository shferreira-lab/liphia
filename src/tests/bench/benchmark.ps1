# ===================================================================
# benchmark.ps1
# Benchmark fib(30) - Liphia / Python / Java / Node.js
# Measurement: best of 3 runs, external stopwatch
# ===================================================================

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Benchmark fib(30) - Liphia / Python / Java / Node.js"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$results = @()

function Run-BestOf3 {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    $times = @()
    $lastOutput = $null

    for ($i = 0; $i -lt 3; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $lastOutput = & $Command 2>&1
        $sw.Stop()
        $times += $sw.Elapsed.TotalMilliseconds
    }

    $best = [math]::Round(($times | Measure-Object -Minimum).Minimum, 2)
    return @{ Best = $best; Output = $lastOutput }
}

# ===================================================================
# LIPHiA
# ===================================================================
Write-Host "Running Liphia..." -ForegroundColor Yellow
$liphia_cli = "C:\Dev\liphia\src\target\release\liphia_cli.exe"

if (Test-Path $liphia_cli) {
    $r = Run-BestOf3 "Liphia" { & $liphia_cli bench_fib_ai.lph }
    Write-Host "  $($r.Output -join ' | ')"
    Write-Host "  best of 3: $($r.Best) ms"

    $results += [PSCustomObject]@{
        Language = "Liphia (VM)"
        Time_ms  = $r.Best
        Notes    = "best of 3 (external)"
    }
} else {
    Write-Host "  [ERROR] Not found: $liphia_cli" -ForegroundColor Red
}

# ===================================================================
# PYTHON
# ===================================================================
Write-Host "Running Python..." -ForegroundColor Yellow
$py = Get-Command python -ErrorAction SilentlyContinue

if ($py) {
    $r = Run-BestOf3 "Python" { & python bench_fib_ai.py }
    Write-Host "  $($r.Output -join ' | ')"
    Write-Host "  best of 3: $($r.Best) ms"

    $results += [PSCustomObject]@{
        Language = "Python 3"
        Time_ms  = $r.Best
        Notes    = "best of 3 (external)"
    }
} else {
    Write-Host "  [SKIP] Python not found" -ForegroundColor DarkGray
}

# ===================================================================
# JAVA
# ===================================================================
Write-Host "Running Java..." -ForegroundColor Yellow
$javac = Get-Command javac -ErrorAction SilentlyContinue
$java  = Get-Command java  -ErrorAction SilentlyContinue

if ($javac -and $java) {
    & javac BenchFibAI.java 2>&1 | Out-Null

    $r = Run-BestOf3 "Java" { & java BenchFibAI }
    Write-Host "  $($r.Output -join ' | ')"
    Write-Host "  best of 3: $($r.Best) ms"

    $results += [PSCustomObject]@{
        Language = "Java (JVM)"
        Time_ms  = $r.Best
        Notes    = "best of 3 (external)"
    }
} else {
    Write-Host "  [SKIP] Java not found" -ForegroundColor DarkGray
}

# ===================================================================
# NODE.JS
# ===================================================================
Write-Host "Running Node.js..." -ForegroundColor Yellow
$node = Get-Command node -ErrorAction SilentlyContinue

if ($node) {
    $r = Run-BestOf3 "Node.js" { & node bench_fib_ai.js }
    Write-Host "  $($r.Output -join ' | ')"
    Write-Host "  best of 3: $($r.Best) ms"

    $results += [PSCustomObject]@{
        Language = "Node.js (V8)"
        Time_ms  = $r.Best
        Notes    = "best of 3 (external)"
    }
} else {
    Write-Host "  [SKIP] Node.js not found" -ForegroundColor DarkGray
}

# ===================================================================
# FINAL RESULTS
# ===================================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Final Results"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$results | Sort-Object Time_ms | Format-Table -AutoSize

Write-Host ""
Write-Host "All timings were measured externally using Stopwatch." -ForegroundColor DarkGray
Write-Host "Method: best of 3 runs." -ForegroundColor DarkGray
Write-Host ""