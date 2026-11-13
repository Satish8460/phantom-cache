# Reproduce the test suite and summarize it (PowerShell). New in M7; does not modify the tree.
# Usage: scripts\reproduce_tests.ps1 [-LogDir repro-logs]
# Honors CARGO_TARGET_DIR. Always uses -j 1 (project rule).
param([string]$LogDir = 'repro-logs')
$ErrorActionPreference = 'Continue'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $root
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$log = Join-Path $LogDir 'cargo_test_all_targets.log'
@(
    "date_utc=$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))",
    "rustc=$(& rustc --version)",
    "head=$(& git rev-parse HEAD 2>$null)",
    "dirty_files=$((& git status --short 2>$null | Measure-Object).Count)"
) | Set-Content (Join-Path $LogDir 'environment.txt')

cmd /c "cargo test --all-targets -j 1 > `"$log`" 2>&1"
$rc = $LASTEXITCODE
$passed = 0; $failed = 0; $ignored = 0; $targets = 0
foreach ($l in Get-Content $log) {
    if ($l -match '^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;') {
        $targets++; $passed += [int]$Matches[1]; $failed += [int]$Matches[2]; $ignored += [int]$Matches[3]
    }
}
Write-Host "cargo test exit code: $rc"
Write-Host "targets with results: $targets  passed: $passed  failed: $failed  ignored: $ignored"
Write-Host "full log: $log"
Write-Host 'NOTE: cargo test --all-targets also executes the harness=false bench mains; see docs/release/ci_recommendations.md'
exit $rc
