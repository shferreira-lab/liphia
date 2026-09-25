# packages/build.ps1
#
# Builds every native package in release mode and copies its library into
# packages/<name>/lib/, where `liphia install` downloads it from.
# Run from anywhere; the script locates the workspace (src/) itself.
#
#   powershell -ExecutionPolicy Bypass -File src\packages\build.ps1

$ErrorActionPreference = "Stop"
$packages = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspace = Split-Path -Parent $packages
$names = @("db", "num", "stats", "learn")

Push-Location $workspace
try {
    foreach ($name in $names) {
        Write-Host "[build] liphia_package_$name"
        cargo build -p "liphia_package_$name" --release
        if ($LASTEXITCODE -ne 0) { throw "build failed: $name" }

        $lib = Join-Path $packages "$name\lib"
        New-Item -ItemType Directory -Force -Path $lib | Out-Null
        Copy-Item "target\release\liphia_package_$name.dll" $lib -Force
    }
    Write-Host "[build] done: libraries copied to packages\<name>\lib\"
}
finally {
    Pop-Location
}
