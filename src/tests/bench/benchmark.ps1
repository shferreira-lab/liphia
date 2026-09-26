# benchmark.ps1
#
# Runs every workload in every available language, times each whole process
# externally (best of N runs), checks that all languages print the same
# checksum, and prints a table. The table is also saved as
# results-<date>.md next to this script.
#
#   powershell -ExecutionPolicy Bypass -File .\benchmark.ps1
#   powershell -ExecutionPolicy Bypass -File .\benchmark.ps1 -Runs 5 -Only fib,sort
#
# Liphia is taken from PATH (`liphia`) unless -Liphia points to an executable.

param(
    [int]$Runs = 3,
    [string[]]$Only = @(),
    [string]$Liphia = "liphia"
)

# Continue, not Stop: Windows PowerShell 5.1 turns any stderr output of a
# redirected native command (Liphia's cache notice, `java -version`) into an
# error record, which "Stop" would make fatal.
$ErrorActionPreference = "Continue"
Set-Location $PSScriptRoot

$Workloads = @(
    @{ Name = "startup";    File = "00_startup.lph" },
    @{ Name = "fib";        File = "01_fib.lph" },
    @{ Name = "loops";      File = "02_loops.lph" },
    @{ Name = "mandelbrot"; File = "03_mandelbrot.lph" },
    @{ Name = "sieve";      File = "04_sieve.lph" },
    @{ Name = "sort";       File = "05_sort.lph" },
    @{ Name = "map";        File = "06_map.lph" }
)
# With -File, "-Only fib,sort" arrives as one string; split it here.
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ -ne "" })
if ($Only.Count -gt 0) {
    $Workloads = @($Workloads | Where-Object { $Only -contains $_.Name })
}

# Each language: display name and a block that runs one workload.
$Languages = [ordered]@{}

if (Get-Command $Liphia -ErrorAction SilentlyContinue) {
    $Languages["Liphia"] = { param($w) & $Liphia "liphia/$($w.File)" }
} else {
    Write-Host "Liphia not found ('$Liphia'). Open a new terminal after installing, or pass -Liphia <path to liphia.exe>." -ForegroundColor Yellow
}
$python = @("python", "python3") | Where-Object { Get-Command $_ -ErrorAction SilentlyContinue } | Select-Object -First 1
if ($python) {
    $Languages["Python"] = { param($w) & $python "python/bench.py" $w.Name }
}
if (Get-Command node -ErrorAction SilentlyContinue) {
    $Languages["Node.js"] = { param($w) & node "node/bench.js" $w.Name }
}
if ((Get-Command javac -ErrorAction SilentlyContinue) -and (Get-Command java -ErrorAction SilentlyContinue)) {
    & javac -d java java/Bench.java
    $Languages["Java"] = { param($w) & java -cp java Bench $w.Name }
}

if ($Languages.Count -eq 0) {
    throw "no language found (liphia, python, node, java)"
}

Write-Host ""
Write-Host "Liphia benchmark - best of $Runs runs, external timing" -ForegroundColor Cyan
Write-Host "Languages: $($Languages.Keys -join ', ')"
Write-Host ""

# Warm-up: one untimed run per language fills caches (Liphia bytecode,
# OS file cache) so the first timed run is not penalized.
foreach ($lang in $Languages.Keys) {
    & $Languages[$lang] $Workloads[0] 2>$null | Out-Null
}

$rows = @()
$totals = @{}
$mismatch = $false

foreach ($w in $Workloads) {
    $row = [ordered]@{ Workload = $w.Name }
    $reference = $null
    foreach ($lang in $Languages.Keys) {
        $best = [double]::MaxValue
        $output = ""
        for ($i = 0; $i -lt $Runs; $i++) {
            $sw = [Diagnostics.Stopwatch]::StartNew()
            # Only stdout is compared; Liphia reports cache use on stderr.
            $output = (& $Languages[$lang] $w 2>$null | Out-String).Trim()
            $sw.Stop()
            $best = [math]::Min($best, $sw.Elapsed.TotalMilliseconds)
        }
        if ($null -eq $reference) { $reference = $output }
        $mark = ""
        if ($output -ne $reference) {
            $mark = " !"
            $mismatch = $true
            Write-Host "  [$lang] $($w.Name): '$output' differs from '$reference'" -ForegroundColor Red
        }
        $row[$lang] = "{0:N0}{1}" -f $best, $mark
        if ($w.Name -ne "startup") { $totals[$lang] += $best }
        Write-Host ("  {0,-11} {1,-8} {2,10:N0} ms   {3}" -f $w.Name, $lang, $best, $output)
    }
    $rows += [pscustomobject]$row
}

$totalRow = [ordered]@{ Workload = "TOTAL (no startup)" }
foreach ($lang in $Languages.Keys) { $totalRow[$lang] = "{0:N0}" -f $totals[$lang] }
$rows += [pscustomobject]$totalRow

Write-Host ""
$rows | Format-Table -AutoSize
Write-Host "Times in ms, whole process (start + compile + run), best of $Runs."
if ($mismatch) {
    Write-Host "Rows marked '!' printed a different checksum: that result is invalid." -ForegroundColor Red
}

# Markdown copy of the table.
$date = Get-Date -Format "yyyy-MM-dd"
$langs = @($Languages.Keys)
$md = @("# Liphia benchmark - $date", "",
        "Best of $Runs runs, whole process timed externally (ms). Machine: $env:COMPUTERNAME, $([Environment]::OSVersion.VersionString).", "",
        "| Workload | $($langs -join ' | ') |",
        "|---|$(($langs | ForEach-Object { '---:' }) -join '|')|")
foreach ($r in $rows) {
    $md += "| $($r.Workload) | $(($langs | ForEach-Object { $r.$_ }) -join ' | ') |"
}
$versions = @()
# `python --version` and `java -version` print to stderr; "$(...)" turns
# the captured records into plain text.
if ($Languages.Contains("Liphia")) { $versions += "- Liphia: $(& $Liphia version 2>&1 | Select-Object -First 1)" }
if ($python) { $versions += "- Python: $(& $python --version 2>&1 | Select-Object -First 1)" }
if ($Languages.Contains("Node.js")) { $versions += "- Node.js: $(& node --version 2>&1 | Select-Object -First 1)" }
if ($Languages.Contains("Java")) { $versions += "- Java: $(& java -version 2>&1 | Select-Object -First 1)" }
$md += @("", "Versions:", "") + $versions
$file = Join-Path $PSScriptRoot "results-$date.md"
$md -join "`n" | Set-Content -Encoding utf8 $file
Write-Host "Saved $file"