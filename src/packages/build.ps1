# packages/build.ps1
#
# Builds native packages in release mode and copies each library into
# packages/<n>/lib/, where `liphia install` downloads it from.
#
# A package is native when its package.toml has a [native] section; the
# library name is the `lib` key of that section. With no arguments every
# native package is built; otherwise only the names given.
# Run from anywhere; the script locates the workspace (src/) itself.
#
#   powershell -ExecutionPolicy Bypass -File src\packages\build.ps1
#   powershell -ExecutionPolicy Bypass -File src\packages\build.ps1 crypto num

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Names
)

$ErrorActionPreference = "Stop"
$packages = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspace = Split-Path -Parent $packages

# Returns the `lib` value of the [native] section, or $null when the
# package.toml has no such section.
function Get-NativeLib([string]$toml) {
    $inNative = $false
    foreach ($line in Get-Content $toml) {
        $text = $line.Trim()
        if ($text -match '^\[([A-Za-z0-9_.-]+)\]$') {
            $inNative = ($Matches[1] -eq "native")
            continue
        }
        if ($inNative -and $text -match '^lib\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }
    return $null
}

# Maps package name -> library name for every native package found.
$native = [ordered]@{}
foreach ($dir in Get-ChildItem -Path $packages -Directory) {
    $toml = Join-Path $dir.FullName "package.toml"
    if (-not (Test-Path $toml)) { continue }
    $lib = Get-NativeLib $toml
    if ($lib) { $native[$dir.Name] = $lib }
}

if ($native.Count -eq 0) { throw "no native packages found in $packages" }

if ($Names -and $Names.Count -gt 0) {
    foreach ($name in $Names) {
        if (-not $native.Contains($name)) { throw "not a native package: $name" }
    }
    $targets = $Names
}
else {
    $targets = @($native.Keys)
}

Push-Location $workspace
try {
    foreach ($name in $targets) {
        $lib = $native[$name]
        Write-Host "[build] $lib"
        cargo build -p $lib --release
        if ($LASTEXITCODE -ne 0) { throw "build failed: $name" }

        $dest = Join-Path $packages "$name\lib"
        New-Item -ItemType Directory -Force -Path $dest | Out-Null
        Copy-Item "target\release\$lib.dll" $dest -Force
    }
    Write-Host "[build] done: libraries copied to packages\<n>\lib\"
}
finally {
    Pop-Location
}