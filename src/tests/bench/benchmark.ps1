# ===================================================================
# benchmark.ps1
# Benchmark fib(30) - Liphia / Python / Java / Node.js
# Medicao: melhor de 3 execucoes, stopwatch externo
# ===================================================================

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Benchmark fib(30) - Liphia / Python / Java / Node.js"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$resultados = @()

function Rodar-MelhorDe3 {
    param(
        [string]$Nome,
        [scriptblock]$Cmd
    )

    $tempos = @()
    $ultimaSaida = $null

    for ($i = 0; $i -lt 3; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $ultimaSaida = & $Cmd 2>&1
        $sw.Stop()
        $tempos += $sw.Elapsed.TotalMilliseconds
    }

    $melhor = [math]::Round(($tempos | Measure-Object -Minimum).Minimum, 2)
    return @{ Melhor = $melhor; Saida = $ultimaSaida }
}

# ===================================================================
# LIPHiA
# ===================================================================
Write-Host "Rodando Liphia..." -ForegroundColor Yellow
$liphia_cli = "C:\Dev\liphia\liphia_engine\target\release\liphia_cli.exe"

if (Test-Path $liphia_cli) {
    $r = Rodar-MelhorDe3 "Liphia" { & $liphia_cli bench_fib_ai.lph }
    Write-Host "  $($r.Saida -join ' | ')"
    Write-Host "  melhor de 3: $($r.Melhor) ms"

    $resultados += [PSCustomObject]@{
        Linguagem = "Liphia (VM)"
        Tempo_ms  = $r.Melhor
        Nota      = "melhor de 3 (externo)"
    }
} else {
    Write-Host "  [ERRO] nao encontrado: $liphia_cli" -ForegroundColor Red
}

# ===================================================================
# PYTHON
# ===================================================================
Write-Host "Rodando Python..." -ForegroundColor Yellow
$py = Get-Command python -ErrorAction SilentlyContinue

if ($py) {
    $r = Rodar-MelhorDe3 "Python" { & python bench_fib_ai.py }
    Write-Host "  $($r.Saida -join ' | ')"
    Write-Host "  melhor de 3: $($r.Melhor) ms"

    $resultados += [PSCustomObject]@{
        Linguagem = "Python 3"
        Tempo_ms  = $r.Melhor
        Nota      = "melhor de 3 (externo)"
    }
} else {
    Write-Host "  [SKIP] python nao encontrado" -ForegroundColor DarkGray
}

# ===================================================================
# JAVA
# ===================================================================
Write-Host "Rodando Java..." -ForegroundColor Yellow
$javac = Get-Command javac -ErrorAction SilentlyContinue
$java  = Get-Command java  -ErrorAction SilentlyContinue

if ($javac -and $java) {
    & javac BenchFibAI.java 2>&1 | Out-Null

    $r = Rodar-MelhorDe3 "Java" { & java BenchFibAI }
    Write-Host "  $($r.Saida -join ' | ')"
    Write-Host "  melhor de 3: $($r.Melhor) ms"

    $resultados += [PSCustomObject]@{
        Linguagem = "Java (JVM)"
        Tempo_ms  = $r.Melhor
        Nota      = "melhor de 3 (externo)"
    }
} else {
    Write-Host "  [SKIP] javac/java nao encontrado" -ForegroundColor DarkGray
}

# ===================================================================
# NODE.JS
# ===================================================================
Write-Host "Rodando Node.js..." -ForegroundColor Yellow
$node = Get-Command node -ErrorAction SilentlyContinue

if ($node) {
    $r = Rodar-MelhorDe3 "Node.js" { & node bench_fib_ai.js }
    Write-Host "  $($r.Saida -join ' | ')"
    Write-Host "  melhor de 3: $($r.Melhor) ms"

    $resultados += [PSCustomObject]@{
        Linguagem = "Node.js (V8)"
        Tempo_ms  = $r.Melhor
        Nota      = "melhor de 3 (externo)"
    }
} else {
    Write-Host "  [SKIP] node nao encontrado" -ForegroundColor DarkGray
}

# ===================================================================
# RESULTADO FINAL
# ===================================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Resultado final                       " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$resultados | Sort-Object Tempo_ms | Format-Table -AutoSize

Write-Host ""
Write-Host "Todos os tempos: medicao EXTERNA (Stopwatch)." -ForegroundColor DarkGray
Write-Host "Metodo: melhor de 3 execucoes." -ForegroundColor DarkGray
Write-Host ""