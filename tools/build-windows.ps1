param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$SkipRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

$env:RUST_BACKTRACE = "1"

Write-Host "Repository: $RepoRoot"
Write-Host "Target: $Target"
cargo --version

cargo build -p wecode -p wecode-wrapper-helper --release --target $Target

$releaseDir = Join-Path $RepoRoot "target\$Target\release"

if (-not $SkipRun) {
    $exe = Join-Path $releaseDir "wecode.exe"
    if (-not (Test-Path $exe)) {
        throw "Built executable was not found: $exe"
    }
    & $exe --version

    $helper = Join-Path $releaseDir "wecode-wrapper-helper.exe"
    if (-not (Test-Path $helper)) {
        throw "Built wrapper helper was not found: $helper"
    }
    $profiles = Join-Path $env:TEMP "wecode-empty-ssh-profiles.json"
    [System.IO.File]::WriteAllText($profiles, "null", [System.Text.UTF8Encoding]::new($false))
    $env:WECODE_SSH_PROFILES_FILE = $profiles
    & $helper --wecode-wrapper-helper ssh-list-profiles
    if ($LASTEXITCODE -ne 0) {
        throw "wrapper helper smoke failed with exit code $LASTEXITCODE"
    }
}
