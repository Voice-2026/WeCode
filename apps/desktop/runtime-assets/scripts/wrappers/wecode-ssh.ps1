$ErrorActionPreference = "Stop"

$wrapperDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if ($wrapperDir.StartsWith('\\?\UNC\')) { $wrapperDir = '\\' + $wrapperDir.Substring(8) }
elseif ($wrapperDir.StartsWith('\\?\')) { $wrapperDir = $wrapperDir.Substring(4) }

$helper = Join-Path $wrapperDir "wecode-wrapper-helper.exe"

function Usage {
  Write-Output "usage: wecode-ssh list"
  Write-Output "       wecode-ssh <profile-id>"
  Write-Output "       wecode-ssh <profile-id> -- '<remote-command>'"
  Write-Output "       wecode-ssh scp <profile-id> <src> <dst>   (mark the remote path with a leading ':', e.g. :/etc/hosts)"
  Write-Output ""
  Write-Output "Use 'wecode-ssh list' to read saved SSH profiles as JSON."
}

function Resolve-Profiles-File {
  $profilesFile = $env:WECODE_SSH_PROFILES_FILE
  if ([string]::IsNullOrWhiteSpace($profilesFile) -and -not [string]::IsNullOrWhiteSpace($env:DMUX_APP_SUPPORT_ROOT)) {
    $profilesFile = Join-Path $env:DMUX_APP_SUPPORT_ROOT "ssh_profiles.json"
  }
  if ([string]::IsNullOrWhiteSpace($profilesFile)) {
    $profilesFile = Join-Path $env:APPDATA "WeCode\ssh_profiles.json"
  }
  if (-not (Test-Path -LiteralPath $profilesFile)) {
    [Console]::Error.WriteLine("wecode-ssh: unable to read SSH profile file")
    exit 66
  }
  return $profilesFile
}

function Require-Helper {
  if (-not (Test-Path -LiteralPath $helper)) {
    [Console]::Error.WriteLine("wecode-ssh: bundled helper is missing")
    exit 127
  }
}

function Invoke-Helper([string]$Subcommand) {
  Require-Helper
  & $helper --wecode-wrapper-helper $Subcommand
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}

function Find-Executable([string]$Name) {
  $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($command -and $command.Source) {
    return $command.Source
  }
  return $Name
}

function Convert-Helper-Assignment([string]$Line) {
  $separator = $Line.IndexOf("=")
  if ($separator -le 0) { return $null }
  $name = $Line.Substring(0, $separator).Trim()
  $value = $Line.Substring($separator + 1).Trim()
  if ($value.StartsWith("'") -and $value.EndsWith("'") -and $value.Length -ge 2) {
    # PS double-quoted strings do not escape backslash: "'\''" is the literal 4-char POSIX quote escape.
    $value = $value.Substring(1, $value.Length - 2).Replace("'\''", "'")
  }
  return @{ name = $name; value = $value }
}

function Split-Shell-Array([string]$Value) {
  $value = $Value.Trim()
  if ($value.StartsWith("(") -and $value.EndsWith(")")) {
    $value = $value.Substring(1, $value.Length - 2)
  }
  $matches = [Regex]::Matches($value, "'((?:'\\''|[^'])*)'|([^\s]+)")
  $items = @()
  foreach ($match in $matches) {
    if ($match.Groups[1].Success) {
      $items += $match.Groups[1].Value.Replace("'\''", "'")
    } elseif ($match.Groups[2].Success) {
      $items += $match.Groups[2].Value
    }
  }
  return $items
}

function Read-Ssh-Profile-Shell([string]$ProfileId, [string]$ProfilesFile) {
  Require-Helper
  $env:WECODE_SSH_PROFILE_ID = $ProfileId
  $env:WECODE_SSH_PROFILES_FILE = $ProfilesFile
  $lines = @(& $helper --wecode-wrapper-helper ssh-profile-shell)
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
  $result = @{
    ssh_password = ""
    ssh_key_passphrase = ""
    ssh_args = @()
  }
  foreach ($line in $lines) {
    $assignment = Convert-Helper-Assignment $line
    if ($null -eq $assignment) { continue }
    switch ($assignment.name) {
      "ssh_password" { $result.ssh_password = $assignment.value }
      "ssh_key_passphrase" { $result.ssh_key_passphrase = $assignment.value }
      "ssh_args" { $result.ssh_args = Split-Shell-Array $assignment.value }
    }
  }
  return $result
}

function Read-Scp-Profile-Shell([string]$ProfileId, [string]$ProfilesFile) {
  Require-Helper
  $env:WECODE_SSH_PROFILE_ID = $ProfileId
  $env:WECODE_SSH_PROFILES_FILE = $ProfilesFile
  $lines = @(& $helper --wecode-wrapper-helper scp-profile-shell)
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
  $result = @{
    ssh_password = ""
    ssh_key_passphrase = ""
    ssh_remote = ""
    scp_args = @()
  }
  foreach ($line in $lines) {
    $assignment = Convert-Helper-Assignment $line
    if ($null -eq $assignment) { continue }
    switch ($assignment.name) {
      "ssh_password" { $result.ssh_password = $assignment.value }
      "ssh_key_passphrase" { $result.ssh_key_passphrase = $assignment.value }
      "ssh_remote" { $result.ssh_remote = $assignment.value }
      "scp_args" { $result.scp_args = Split-Shell-Array $assignment.value }
    }
  }
  return $result
}

function Set-Ssh-AskPass-Environment([hashtable]$Profile) {
  if ([string]::IsNullOrWhiteSpace([string]$Profile.ssh_password) -and
      [string]::IsNullOrWhiteSpace([string]$Profile.ssh_key_passphrase)) {
    return
  }
  Require-Helper
  $env:WECODE_WRAPPER_HELPER_ASKPASS = "1"
  $env:WECODE_SSH_PASSWORD = [string]$Profile.ssh_password
  $env:WECODE_SSH_KEY_PASSPHRASE = [string]$Profile.ssh_key_passphrase
  $env:SSH_ASKPASS = $helper
  $env:SSH_ASKPASS_REQUIRE = "force"
  $env:DISPLAY = if ([string]::IsNullOrWhiteSpace($env:DISPLAY)) { "wecode" } else { $env:DISPLAY }
}

# The script runs in the caller's PowerShell process; secrets and the forced
# askpass must not outlive the ssh/scp child (mirrors the zsh wrapper's unset).
function Clear-Ssh-AskPass-Environment {
  Remove-Item Env:WECODE_WRAPPER_HELPER_ASKPASS -ErrorAction SilentlyContinue
  Remove-Item Env:WECODE_SSH_PASSWORD -ErrorAction SilentlyContinue
  Remove-Item Env:WECODE_SSH_KEY_PASSPHRASE -ErrorAction SilentlyContinue
  Remove-Item Env:SSH_ASKPASS -ErrorAction SilentlyContinue
  Remove-Item Env:SSH_ASKPASS_REQUIRE -ErrorAction SilentlyContinue
  if ($env:DISPLAY -eq "wecode") { Remove-Item Env:DISPLAY -ErrorAction SilentlyContinue }
}

$argv = @($args | ForEach-Object { [string]$_ })
$command = if ($argv.Count -gt 0) { $argv[0] } else { "" }

if ($command -eq "-h" -or $command -eq "--help" -or $command -eq "help") {
  Usage
  exit 0
}

if ($command -eq "list" -or $command -eq "--list" -or $command -eq "profiles") {
  if ($argv.Count -gt 1) {
    [Console]::Error.WriteLine("usage: wecode-ssh list")
    exit 64
  }
  $profilesFile = Resolve-Profiles-File
  $env:WECODE_SSH_PROFILES_FILE = $profilesFile
  Invoke-Helper "ssh-list-profiles"
  exit $LASTEXITCODE
}

if ($command -eq "scp") {
  if ($argv.Count -lt 4) {
    [Console]::Error.WriteLine("usage: wecode-ssh scp <profile-id> <src> <dst>  (mark the remote path with a leading ':', e.g. :/etc/hosts)")
    exit 64
  }
  $profilesFile = Resolve-Profiles-File
  $profileId = $argv[1]
  $profile = Read-Scp-Profile-Shell $profileId $profilesFile
  if ($profile.scp_args.Count -eq 0 -or [string]::IsNullOrWhiteSpace([string]$profile.ssh_remote)) {
    [Console]::Error.WriteLine("wecode-ssh: invalid SSH profile")
    exit 65
  }
  $scpArgs = @($profile.scp_args)
  $scpArgs[0] = Find-Executable "scp"
  foreach ($operand in @($argv[2..($argv.Count - 1)])) {
    if ($operand.StartsWith(":")) {
      $scpArgs += "$($profile.ssh_remote):$($operand.Substring(1))"
    } else {
      $scpArgs += $operand
    }
  }
  Set-Ssh-AskPass-Environment $profile
  try {
    & $scpArgs[0] @($scpArgs | Select-Object -Skip 1)
    $exitCode = $LASTEXITCODE
  } finally {
    Clear-Ssh-AskPass-Environment
  }
  exit $exitCode
}

if ([string]::IsNullOrWhiteSpace($command)) {
  [Console]::Error.WriteLine("wecode-ssh: missing profile id")
  exit 64
}

$profileId = $command
$remoteArgs = @()
if ($argv.Count -gt 1) {
  if ($argv[1] -ne "--") {
    [Console]::Error.WriteLine("usage: wecode-ssh <profile-id> [-- <remote-command>] | wecode-ssh list")
    exit 64
  }
  if ($argv.Count -lt 3) {
    [Console]::Error.WriteLine("wecode-ssh: missing remote command after --")
    exit 64
  }
  $remoteArgs = @($argv[2..($argv.Count - 1)])
}

$profilesFile = Resolve-Profiles-File
$profile = Read-Ssh-Profile-Shell $profileId $profilesFile
if ($profile.ssh_args.Count -eq 0) {
  [Console]::Error.WriteLine("wecode-ssh: invalid SSH profile")
  exit 65
}
$sshArgs = @($profile.ssh_args)
$sshArgs[0] = Find-Executable "ssh"
$sshArgs += $remoteArgs
Set-Ssh-AskPass-Environment $profile
try {
  & $sshArgs[0] @($sshArgs | Select-Object -Skip 1)
  $exitCode = $LASTEXITCODE
} finally {
  Clear-Ssh-AskPass-Environment
}
exit $exitCode
