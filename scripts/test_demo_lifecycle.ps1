# test_demo_lifecycle.ps1
# Automates 3 complete lifecycle verification cycles for Start-Phantom-Demo.bat
# Validates: START -> CONNECT -> USE -> STOP -> VERIFY CLEANUP -> RESTART

$ErrorActionPreference = "Stop"

function Assert-PortsFree {
    param([int[]]$ports = @(8085, 3000))
    foreach ($p in $ports) {
        try {
            $tcp = New-Object System.Net.Sockets.TcpClient
            $tcp.Connect("127.0.0.1", $p)
            $tcp.Close()
            throw "Assertion failed: Port $p is still accepting connections on 127.0.0.1!"
        } catch [System.Net.Sockets.SocketException] {
            # Expected: ConnectionRefused means port is completely free
        }
    }
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "PHANTOMCACHE DEMO LAUNCHER MULTI-CYCLE VALIDATION" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Pre-flight check: ensure ports are clean
Assert-PortsFree
Write-Host "Pre-flight check passed: ports 8085 and 3000 are clean." -ForegroundColor Green

$cycles = 3
for ($i = 1; $i -le $cycles; $i++) {
    Write-Host "`n>>> [CYCLE $i of $cycles] Starting Launcher in no-browser mode..." -ForegroundColor Yellow
    
    $pinfo = New-Object System.Diagnostics.ProcessStartInfo
    $batPath = (Resolve-Path "Start-Phantom-Demo.bat").Path
    $pinfo.FileName = "cmd.exe"
    $pinfo.Arguments = "/c `"$batPath`" --no-browser"
    $pinfo.UseShellExecute = $false
    $pinfo.RedirectStandardInput = $true
    $pinfo.RedirectStandardOutput = $true
    $pinfo.RedirectStandardError = $true
    $pinfo.WorkingDirectory = (Get-Location).Path

    $proc = [System.Diagnostics.Process]::Start($pinfo)
    
    # Monitor output for the wait prompt
    $ready = $false
    $timeoutSeconds = 45
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $launcherOutput = [System.Text.StringBuilder]::new()

    while ($sw.Elapsed.TotalSeconds -lt $timeoutSeconds -and -not $proc.HasExited) {
        $line = $proc.StandardOutput.ReadLine()
        if ($null -ne $line) {
            [void]$launcherOutput.AppendLine($line)
            Write-Host "  [BAT] $line" -ForegroundColor DarkGray
            if ($line -match "Press any key to stop all background processes") {
                $ready = $true
                break
            }
        }
    }

    if (-not $ready) {
        Write-Host "Launcher stdout dump:" -ForegroundColor Red
        Write-Host $launcherOutput.ToString()
        $errOut = $proc.StandardError.ReadToEnd()
        Write-Host "Launcher stderr dump:" -ForegroundColor Red
        Write-Host $errOut
        if (-not $proc.HasExited) { $proc.Kill() }
        throw "Cycle $i failed: Launcher did not reach SYSTEM READY within $timeoutSeconds seconds."
    }

    Write-Host "  [OK] Cycle $($i): Launcher is ready and awaiting signal." -ForegroundColor Green

    # Exercise live backend endpoints
    Write-Host "  [HTTP] Probing http://localhost:8085/health..." -ForegroundColor Yellow
    $healthResp = Invoke-RestMethod -Uri "http://localhost:8085/health" -Method Get -TimeoutSec 5
    if ($healthResp -ne "OK") {
        throw "Cycle $i failed: Health response was not OK: $healthResp"
    }
    Write-Host "  [OK] Health probe: $healthResp" -ForegroundColor Green

    Write-Host "  [HTTP] Probing http://localhost:8085/api/v1/metrics..." -ForegroundColor Yellow
    $metricsResp = Invoke-RestMethod -Uri "http://localhost:8085/api/v1/metrics" -Method Get -TimeoutSec 5
    if ($null -eq $metricsResp.metrics) {
        throw "Cycle $i failed: Metrics endpoint did not return metrics object"
    }
    Write-Host "  [OK] Metrics probe: valid metrics schema received." -ForegroundColor Green

    # Exercise live frontend endpoint with bounded retries for Vite HTML transform
    Write-Host "  [HTTP] Probing http://localhost:3000 (Vite frontend)..." -ForegroundColor Yellow
    $uiSuccess = $false
    $uiText = ""
    for ($attempt = 1; $attempt -le 5; $attempt++) {
        $uiHtml = curl.exe -s http://localhost:3000
        $uiText = $uiHtml -join "`n"
        if ($uiText -match "root") {
            $uiSuccess = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $uiSuccess) {
        throw "Cycle $i failed: UI returned unexpected response after 5 attempts: $uiText"
    }
    Write-Host "  [OK] UI probe: HTTP 200 OK with root container." -ForegroundColor Green

    # Stop the launcher by writing Enter to its StandardInput
    Write-Host "  [STOP] Signaling shutdown to launcher..." -ForegroundColor Yellow
    $proc.StandardInput.WriteLine("")
    $proc.StandardInput.Flush()

    # Wait for process exit
    $exited = $proc.WaitForExit(10000)
    if (-not $exited) {
        Write-Host "  [WARN] Process did not exit in 10s, killing..." -ForegroundColor Red
        $proc.Kill()
    }

    # Verify PID files are removed
    Start-Sleep -Seconds 2
    if (Test-Path .api.pid) {
        throw "Cycle $i failed: .api.pid still exists after shutdown!"
    }
    if (Test-Path .ui.pid) {
        throw "Cycle $i failed: .ui.pid still exists after shutdown!"
    }

    # Verify ports are completely free
    Assert-PortsFree
    Write-Host "  [PASS] Cycle $i completed and verified. Ports clean and sockets released." -ForegroundColor Green
}

# Collision Test: verify collision detection without killing foreign processes
Write-Host "`n>>> [COLLISION TEST] Verifying port collision handling..." -ForegroundColor Yellow
$dummyListener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 8085)
try {
    $dummyListener.Start()
    Write-Host "  Created mock listener on 127.0.0.1:8085" -ForegroundColor DarkGray
    
    $pinfoCol = New-Object System.Diagnostics.ProcessStartInfo
    $batPathCol = (Resolve-Path "Start-Phantom-Demo.bat").Path
    $pinfoCol.FileName = "cmd.exe"
    $pinfoCol.Arguments = "/c `"$batPathCol`" --no-browser"
    $pinfoCol.UseShellExecute = $false
    $pinfoCol.RedirectStandardOutput = $true
    $pinfoCol.RedirectStandardError = $true
    $pinfoCol.WorkingDirectory = (Get-Location).Path

    $procCol = [System.Diagnostics.Process]::Start($pinfoCol)
    $colOut = $procCol.StandardOutput.ReadToEnd()
    $null = $procCol.WaitForExit(10000)
    
    Write-Host "  Collision check output:" -ForegroundColor DarkGray
    Write-Host "  $colOut" -ForegroundColor DarkGray

    if ($procCol.ExitCode -ne 1) {
        throw "Collision test failed: expected exit code 1, got $($procCol.ExitCode)"
    }
    if ($colOut -notmatch "Port 8085 is already occupied") {
        throw "Collision test failed: missing expected warning message."
    }
    Write-Host "  [PASS] Collision detected correctly with exit code 1. No foreign processes terminated." -ForegroundColor Green
} finally {
    $dummyListener.Stop()
}

Start-Sleep -Seconds 1
Assert-PortsFree

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "ALL 3 CYCLES + COLLISION TEST PASSED PERFECTLY!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
