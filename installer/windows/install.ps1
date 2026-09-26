# installer/windows/install.ps1
#
# Installs Liphia for the current user, without admin rights:
#
#   irm https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/windows/install.ps1 | iex
#
# A specific version:
#
#   $env:LIPHIA_VERSION = "2.0.0"; irm https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/windows/install.ps1 | iex
#
# What it does: finds the engine release (tags vX.Y.Z; package releases
# are skipped), downloads liphia-<version>-windows-x86_64.zip, extracts it
# to %LOCALAPPDATA%\Programs\Liphia and adds its bin folder to the user PATH.
# Running it again upgrades in place.

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "shferreira-lab/liphia"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Liphia"
$BinDir = Join-Path $InstallDir "bin"

function Get-EngineVersion {
    if ($env:LIPHIA_VERSION) {
        return $env:LIPHIA_VERSION.TrimStart("v")
    }
    # Releases are listed newest first; engine tags are "v<semver>".
    $releases = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases?per_page=50"
    $engine = $releases | Where-Object { $_.tag_name -match '^v\d+\.\d+\.\d+$' -and -not $_.prerelease } | Select-Object -First 1
    if (-not $engine) {
        throw "no Liphia engine release found in $Repo"
    }
    return $engine.tag_name.TrimStart("v")
}

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "Liphia requires 64-bit Windows"
}

$Version = Get-EngineVersion
$Name = "liphia-$Version-windows-x86_64"
$Url = "https://github.com/$Repo/releases/download/v$Version/$Name.zip"
$Temp = Join-Path ([IO.Path]::GetTempPath()) "liphia-install-$Version"

Write-Host "Installing Liphia $Version"
Write-Host "  from $Url"
Write-Host "  to   $InstallDir"

if (Test-Path $Temp) { Remove-Item $Temp -Recurse -Force }
New-Item -ItemType Directory -Path $Temp | Out-Null
$Zip = Join-Path $Temp "$Name.zip"
Invoke-WebRequest $Url -OutFile $Zip
Expand-Archive $Zip -DestinationPath $Temp -Force

# The archive holds one folder named after itself, with the executables,
# licenses and README at its top level.
$Extracted = Join-Path $Temp $Name
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
Get-ChildItem $Extracted -Filter *.exe | Copy-Item -Destination $BinDir -Force
Get-ChildItem $Extracted -File | Where-Object { $_.Extension -ne ".exe" } | Copy-Item -Destination $InstallDir -Force
Remove-Item $Temp -Recurse -Force

# User PATH, stored in the registry; the current session is updated too so
# `liphia` works right away in this terminal.
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$Entries = @($UserPath -split ";" | Where-Object { $_ -ne "" })
if ($Entries -notcontains $BinDir) {
    $NewPath = (($Entries + $BinDir) -join ";")
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Host "  added $BinDir to the user PATH"
}
if (($env:Path -split ";") -notcontains $BinDir) {
    $env:Path = "$env:Path;$BinDir"
}

Write-Host ""
& (Join-Path $BinDir "liphia.exe") version
Write-Host ""
Write-Host "Liphia $Version is installed. Other terminals and editors (including"
Write-Host "VS Code) must be restarted to see the updated PATH."
