$ErrorActionPreference = "Continue"

Write-Host "Running cargo test --all-targets -j 1..."
$logPath = "bench-results/logs/audit_all_targets.log"
cargo test --all-targets -j 1 > $logPath 2>&1
$exitCode = $LASTEXITCODE

Write-Host "Cargo test exited with code: $exitCode"

$lines = Get-Content $logPath
$targetResults = @()
$currentTarget = ""
$totalPassed = 0
$totalFailed = 0
$totalIgnored = 0

foreach ($line in $lines) {
    if ($line -match "Running (.*)$") {
        $currentTarget = $matches[1]
    }
    elseif ($line -match "test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;") {
        $status = $matches[1]
        $passed = [int]$matches[2]
        $failed = [int]$matches[3]
        $ignored = [int]$matches[4]
        
        $totalPassed += $passed
        $totalFailed += $failed
        $totalIgnored += $ignored

        $targetResults += [PSCustomObject]@{
            Target = $currentTarget
            Status = $status
            Passed = $passed
            Failed = $failed
            Ignored = $ignored
        }
        $currentTarget = ""
    }
}

Write-Host "================ AUDIT SUMMARY ================"
Write-Host "Exit Code: $exitCode"
Write-Host "Total Target Executions: $($targetResults.Count)"
Write-Host "Total Passed: $totalPassed"
Write-Host "Total Failed: $totalFailed"
Write-Host "Total Ignored: $totalIgnored"

$targetResults | Format-Table -AutoSize
