# The Neural Business Engine — Windows setup (one script)
#
# Installs prerequisites via winget, then builds the engine in release mode.
# Run it from PowerShell (right-click > Run with PowerShell, or):
#
#     powershell -ExecutionPolicy Bypass -File engine\scripts\setup-windows.ps1
#
# It is safe to re-run: anything already installed is skipped. If a step fails,
# copy the red error text and send it back — we'll fix it together.

$ErrorActionPreference = 'Stop'

function Have($cmd) { $null -ne (Get-Command $cmd -ErrorAction SilentlyContinue) }

function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user    = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin;C:\Program Files\NASM"
}

function Winget-Install($id, $extra = @()) {
    Write-Host "Installing $id ..." -ForegroundColor Cyan
    winget install --id $id -e --accept-source-agreements --accept-package-agreements @extra
}

Write-Host "== Neural Business Engine: Windows setup ==" -ForegroundColor Green

if (-not (Have winget)) {
    throw "winget not found. Install 'App Installer' from the Microsoft Store, then re-run this script."
}

# 1) Rust toolchain (MSVC)
if (-not (Have cargo)) { Winget-Install 'Rustlang.Rustup' }

# 2) Git (for cloning / updates)
if (-not (Have git)) { Winget-Install 'Git.Git' }

# 3) Visual Studio Build Tools — C++ workload (provides the MSVC compiler & linker)
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$haveVc = (Test-Path $vswhere) -and (& $vswhere -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if (-not $haveVc) {
    Write-Host "Installing Visual Studio 2022 Build Tools (C++)... this is the big one." -ForegroundColor Cyan
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
        --accept-source-agreements --accept-package-agreements `
        --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
}

# 4) Perl + NASM — needed to build the bundled encryption / TLS libraries
if (-not (Have perl)) { Winget-Install 'StrawberryPerl.StrawberryPerl' }
if (-not (Have nasm)) { Winget-Install 'NASM.NASM' }

Refresh-Path

if (-not (Have cargo)) {
    throw "cargo still not found on PATH. Close and reopen PowerShell, then re-run this script."
}

# 5) Build everything (first build is slow — it compiles the whole dependency tree)
$engineDir = Join-Path $PSScriptRoot '..'
Write-Host "Building (release) in $engineDir ..." -ForegroundColor Cyan
Push-Location $engineDir
try {
    cargo build --release
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "All set." -ForegroundColor Green
Write-Host "Next steps (run from the 'engine' folder):" -ForegroundColor Green
Write-Host "  Open the 3D window:   cargo run -p nbe_app --release"
Write-Host "  Use the CLI hub:      .\target\release\nbe.exe --db brain.db stats"
Write-Host "  See all hub commands: .\target\release\nbe.exe --help"
