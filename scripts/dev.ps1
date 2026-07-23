# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

<#
.SYNOPSIS
Runs the repository's Linux development entrypoints from Windows through WSL.

.DESCRIPTION
The hooks and repository scripts depend on Bash, Python 3, and GNU utilities.
This bridge translates the checkout path and delegates to the existing Just
recipes. Unqualified run, readiness, check, test, and lint tasks target the
canonical planet workspace. Old workspace, parked audits, and dawn commands
carry an explicit legacy or parked name. The bridge contains no copied gate or verifier list.
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet(
        "hooks-install",
        "hooks-check",
        "doctor",
        "gates-list",
        "gates-run",
        "gates-self-tests",
        "run",
        "run-derived",
        "readiness",
        "run-dawn-legacy",
        "run-living-legacy",
        "view",
        "view-gpu",
        "view-living-legacy",
        "view-living-gpu-legacy",
        "ledger-inventory",
        "ledger-inventory-check",
        "verify",
        "check-fast",
        "check",
        "check-pr",
        "check-full",
        "check-nightly",
        "check-legacy",
        "ci",
        "ci-local",
        "ci-list",
        "ci-legacy",
        "ci-list-legacy",
        "test",
        "test-gpu-cpu-sparse",
        "test-gpu-vulkan-sparse",
        "test-gpu-cuda-cpu-cross",
        "test-legacy",
        "test-legacy-routine",
        "audit-parked",
        "fmt",
        "fmt-check",
        "fmt-legacy",
        "fmt-check-legacy",
        "lint",
        "lint-legacy",
        "pins-dawn-legacy",
        "stop-gate",
        "cache-info",
        "gc",
        "gc-dry",
        "trim-wsl"
    )]
    [string]$Task = "doctor",

    [string]$Distro = $env:CIVSIM_WSL_DISTRO,

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]]$TaskArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
if (-not $wsl) {
    throw "WSL is required. Install a Linux distribution, then rerun this command."
}

$wslArgs = @()
if (-not [string]::IsNullOrWhiteSpace($Distro)) {
    $wslArgs += @("-d", $Distro)
}

$wslRootRaw = & $wsl.Source @wslArgs -e wslpath -a $repoRoot
$wslPathStatus = $LASTEXITCODE
if ($wslPathStatus -ne 0) {
    exit $wslPathStatus
}
$wslRoot = ($wslRootRaw -join "`n").Trim()
if ([string]::IsNullOrWhiteSpace($wslRoot)) {
    throw "WSL could not translate the repository path."
}

function ConvertTo-BashLiteral {
    param([Parameter(Mandatory = $true)][string]$Value)
    if ($Value.IndexOf([char]39) -ge 0) {
        throw "Repository paths containing a single quote are not supported by this bridge."
    }
    $quote = [char]39
    return "$quote$Value$quote"
}

$commands = @{
    "hooks-install" = "just hooks-install"
    "hooks-check"   = "just hooks-check"
    "gates-list"    = "just gates-list"
    "gates-run"     = "just gates-run"
    "gates-self-tests" = "just gates-self-tests"
    "doctor" = @'
set -euo pipefail
required=(bash python3 git cargo rustc rustup rustfmt just grep awk sed diff mktemp pgrep setsid realpath sha256sum flock)
missing=0
for tool in "${required[@]}"; do
  if command -v "$tool" >/dev/null 2>&1; then
    printf '  OK       %s\n' "$tool"
  else
    printf '  MISSING  %s\n' "$tool" >&2
    missing=1
  fi
done
if [ "$missing" -ne 0 ]; then
  exit 1
fi
rustup show active-toolchain
python3 --version
cargo --version
just doctor
'@
    "run"              = "just run"
    "run-derived"      = "just run-derived"
    "readiness"        = "just readiness"
    "run-dawn-legacy"  = "just run-dawn-legacy"
    "run-living-legacy" = "just run-living-legacy"
    "view"             = "just view"
    "view-gpu"         = "just view-gpu"
    "view-living-legacy" = "just view-living-legacy"
    "view-living-gpu-legacy" = "just view-living-gpu-legacy"
    "ledger-inventory" = "just ledger-inventory"
    "ledger-inventory-check" = "just ledger-inventory-check"
    "verify"           = "just verify"
    "check-fast"       = "just check-fast"
    "check"            = "just check-pr"
    "check-pr"         = "just check-pr"
    "check-full"       = "just check-full"
    "check-nightly"    = "just check-nightly"
    "check-legacy"     = "just check-legacy"
    "ci"               = "just ci"
    "ci-local"         = "just ci-local"
    "ci-list"          = "just ci-list"
    "ci-legacy"        = "just ci-legacy"
    "ci-list-legacy"   = "just ci-list-legacy"
    "test"             = "just test"
    "test-gpu-cpu-sparse" = "just test-gpu-cpu-sparse"
    "test-gpu-vulkan-sparse" = "just test-gpu-vulkan-sparse"
    "test-gpu-cuda-cpu-cross" = "just test-gpu-cuda-cpu-cross"
    "test-legacy"      = "just test-legacy"
    "test-legacy-routine" = "just test-legacy-routine"
    "audit-parked"     = "just audit-parked"
    "fmt"              = "just fmt"
    "fmt-check"        = "just fmt-check"
    "fmt-legacy"       = "just fmt-legacy"
    "fmt-check-legacy" = "just fmt-check-legacy"
    "lint"             = "just lint"
    "lint-legacy"      = "just lint-legacy"
    "pins-dawn-legacy" = "just pins-dawn-legacy"
    "stop-gate"        = "just stop-gate"
    "cache-info"       = "just cache-info"
    "gc"               = "just gc"
    "gc-dry"           = "just gc-dry"
    "trim-wsl"         = "just trim-wsl"
}

$command = $commands[$Task]
if ($TaskArgs.Count -gt 0) {
    $quotedArgs = $TaskArgs | ForEach-Object { ConvertTo-BashLiteral $_ }
    $command = "$command $($quotedArgs -join ' ')"
}

$bashScript = "set -euo pipefail`ncd $(ConvertTo-BashLiteral $wslRoot)`nsource scripts/wsl_dev_env.sh --quiet`n$command"
& $wsl.Source @wslArgs -e bash -lc $bashScript
exit $LASTEXITCODE
