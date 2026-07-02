$ErrorActionPreference = "Stop"
$argv = @($args)
$profileId = if ($argv.Count -gt 0) { [string]$argv[0] } else { "" }
$helper = Join-Path $PSScriptRoot "codux-wrapper-helper.exe"
if (-not (Test-Path $helper)) {
  $helper = Join-Path $PSScriptRoot "codux-wrapper-helper"
}
$jsonOutput = "false"

function Usage {
  Write-Host "usage: codux-db list"
  Write-Host "       codux-db <profile-id> -- '<statement>'"
  Write-Host "       codux-db <profile-id> --file <path>"
}

if ($profileId -eq "-h" -or $profileId -eq "--help" -or $profileId -eq "help") {
  Usage
  exit 0
}

if ($profileId -eq "--json") {
  $jsonOutput = "true"
  $argv = @($argv | Select-Object -Skip 1)
  $profileId = if ($argv.Count -gt 0) { [string]$argv[0] } else { "" }
}

$profilesFile = $env:CODUX_DB_PROFILES_FILE
if ([string]::IsNullOrWhiteSpace($profilesFile) -and -not [string]::IsNullOrWhiteSpace($env:DMUX_APP_SUPPORT_ROOT)) {
  $profilesFile = Join-Path $env:DMUX_APP_SUPPORT_ROOT "db_profiles.json"
}
if ([string]::IsNullOrWhiteSpace($profilesFile)) {
  $profilesFile = Join-Path $env:APPDATA "Codux\db_profiles.json"
}

if ([string]::IsNullOrWhiteSpace($env:CODUX_DB_PROJECT_ID)) {
  Write-Error "codux-db: missing Codux project context"
  exit 64
}
if (-not (Test-Path $profilesFile)) {
  Write-Error "codux-db: unable to read database profile file"
  exit 66
}
if (-not (Test-Path $helper)) {
  Write-Error "codux-db: bundled helper is missing"
  exit 127
}

if ($profileId -eq "list" -or $profileId -eq "--list" -or $profileId -eq "profiles") {
  $env:CODUX_DB_PROFILES_FILE = $profilesFile
  & $helper --codux-wrapper-helper db-list-profiles
  exit $LASTEXITCODE
}

if ([string]::IsNullOrWhiteSpace($profileId)) {
  Write-Error "codux-db: missing profile id"
  exit 64
}

$remaining = @($argv | Select-Object -Skip 1)
if ($remaining.Count -gt 0 -and [string]$remaining[0] -eq "--json") {
  $jsonOutput = "true"
  $remaining = @($remaining | Select-Object -Skip 1)
}

$statement = ""
if ($remaining.Count -gt 0 -and [string]$remaining[0] -eq "--") {
  $remaining = @($remaining | Select-Object -Skip 1)
  if ($remaining.Count -eq 0) {
    Write-Error "codux-db: missing statement after --"
    exit 64
  }
  $statement = ($remaining -join " ")
} elseif ($remaining.Count -gt 0 -and [string]$remaining[0] -eq "--file") {
  if ($remaining.Count -lt 2) {
    Write-Error "codux-db: missing path after --file"
    exit 64
  }
  $statementPath = [string]$remaining[1]
  if (-not (Test-Path $statementPath)) {
    Write-Error "codux-db: unable to read statement file"
    exit 66
  }
  $statement = Get-Content -Raw -LiteralPath $statementPath
} elseif ($remaining.Count -gt 0) {
  Usage
  exit 64
} else {
  Write-Error "codux-db: missing SQL statement"
  exit 64
}

$env:CODUX_DB_PROFILE_ID = $profileId
$env:CODUX_DB_PROFILES_FILE = $profilesFile
$env:CODUX_DB_OUTPUT_JSON = $jsonOutput
$env:CODUX_DB_STATEMENT = $statement
& $helper --codux-wrapper-helper db-query
exit $LASTEXITCODE
