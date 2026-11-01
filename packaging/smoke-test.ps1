# Smoke test of an installed bundle (Windows PowerShell).
# Usage: packaging\smoke-test.ps1 -BundleDir <dir> [-Port 18085]
param([Parameter(Mandatory = $true)][string]$BundleDir, [int]$Port = 18085)
$ErrorActionPreference = 'Stop'
$bin = Join-Path $BundleDir 'phantom-cache-demo.exe'

& $bin --version
& $bin --config (Join-Path $BundleDir 'phantom.example.conf') --check-config | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'check-config on example failed' }
Write-Host 'check-config on example: ok'

$env:PHANTOM_REQUIRE_AUTH = 'yes'
$ErrorActionPreference = 'Continue'   # the binary writes its error to stderr
& $bin --check-config 2>$null | Out-Null
$rc = $LASTEXITCODE
$ErrorActionPreference = 'Stop'
Remove-Item Env:PHANTOM_REQUIRE_AUTH
if ($rc -ne 2) { throw "expected exit code 2 for an invalid boolean, got $rc" }
Write-Host 'invalid configuration rejected (exit 2): ok'

$env:PHANTOM_BIND_ADDR = "127.0.0.1:$Port"
$log = Join-Path $BundleDir 'smoke.log'
$p = Start-Process -FilePath $bin -PassThru -WindowStyle Hidden -RedirectStandardOutput $log -RedirectStandardError "$log.err"
Remove-Item Env:PHANTOM_BIND_ADDR
try {
    $ok = $false
    for ($i = 0; $i -lt 60; $i++) {
        try { $r = Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$Port/health"; $ok = $true; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $ok) { throw 'server did not become healthy' }
    Write-Host "/health: $($r.Content)"
    foreach ($path in '/health/live', '/health/ready') {
        try { $code = (Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$Port$path").StatusCode } catch { $code = $_.Exception.Response.StatusCode.value__ }
        Write-Host "${path}: $code"
    }
    Write-Host 'smoke test passed'
} finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
}
