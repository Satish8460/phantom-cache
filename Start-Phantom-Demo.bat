@echo off
setlocal enabledelayedexpansion
title PHANTOM DEMO LAUNCHER
echo ==============================================
echo               PHANTOM CACHE
echo            INITIALIZING SYSTEM
echo ==============================================

set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"

:: Parse arguments
set NO_BROWSER=0
for %%a in (%*) do (
    if "%%a"=="--no-browser" set NO_BROWSER=1
    if "%%a"=="--headless" set NO_BROWSER=1
)

echo Checking environment prerequisites...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js is not installed or not in PATH.
    pause
    exit /b 1
)

where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] npm is not installed or not in PATH.
    pause
    exit /b 1
)

echo Verifying port availability...
powershell -NoProfile -Command "if (Get-NetTCPConnection -LocalPort 8085 -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"
if %errorlevel%==0 (
    echo [ERROR] Port 8085 is already occupied.
    echo Conflicting listener:
    powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 8085 -ErrorAction SilentlyContinue | Select-Object LocalAddress, LocalPort, OwningProcess, State | Format-Table -AutoSize"
    echo Please ensure no other application or prior PhantomCache instance is using port 8085.
    exit /b 1
)

powershell -NoProfile -Command "if (Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"
if %errorlevel%==0 (
    echo [ERROR] Port 3000 is already occupied.
    echo Conflicting listener:
    powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue | Select-Object LocalAddress, LocalPort, OwningProcess, State | Format-Table -AutoSize"
    echo Please ensure no other application or prior Vite instance is using port 3000.
    exit /b 1
)

:: Binary freshness check
echo Checking binary freshness...
powershell -NoProfile -Command "$bin = 'target\release\phantom-cache-demo.exe'; if (-not (Test-Path $bin)) { exit 1 }; $t = (Get-Item $bin).LastWriteTime; $s = Get-ChildItem -Path src, Cargo.toml -Recurse -File | Where-Object { $_.LastWriteTime -gt $t }; if ($s) { exit 2 } else { exit 0 }"
set BIN_STATUS=%errorlevel%

if %BIN_STATUS%==0 (
    echo [OK] Release binary target\release\phantom-cache-demo.exe is up-to-date.
) else (
    if %BIN_STATUS%==2 (
        echo [INFO] Source files modified since last build. Rebuilding release binary...
    ) else (
        echo [INFO] Release binary missing. Building target\release\phantom-cache-demo.exe...
    )
    set CARGO_INCREMENTAL=0
    cargo build --release --bin phantom-cache-demo -j 1
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to compile release binary.
        pause
        exit /b 1
    )
)

echo Starting Axum API server...
:: Launch API server and record PID
powershell -NoProfile -Command "$p = Start-Process -FilePath 'target\release\phantom-cache-demo.exe' -PassThru -RedirectStandardOutput 'api.log' -RedirectStandardError 'api_err.log'; Set-Content -Path '.api.pid' -Value $p.Id"
if exist .api.pid set /p API_PID=<.api.pid

echo Waiting for API readiness on http://localhost:8085/health...
set HEALTH_RETRIES=0
:poll_health
set /a HEALTH_RETRIES+=1
if %HEALTH_RETRIES% gtr 30 (
    echo [ERROR] Axum API server failed to respond on http://localhost:8085/health within 30 seconds.
    echo Last 20 lines of api_err.log:
    if exist api_err.log powershell -NoProfile -Command "Get-Content -Path api_err.log -Tail 20"
    goto cleanup_and_exit
)
curl -s --connect-timeout 2 --max-time 2 http://localhost:8085/health | findstr OK >nul
if %errorlevel% neq 0 (
    ping 127.0.0.1 -n 2 >nul
    goto poll_health
)
echo [OK] API Server Ready (PID: %API_PID%).

echo Starting React frontend...
:: Launch npm dev server and record PID
powershell -NoProfile -Command "$p = Start-Process -FilePath 'npm.cmd' -ArgumentList 'run dev -- --port 3000' -WorkingDirectory 'ui' -PassThru -RedirectStandardOutput 'ui\ui.log' -RedirectStandardError 'ui\ui_err.log'; Set-Content -Path '.ui.pid' -Value $p.Id"
if exist .ui.pid set /p UI_PID=<.ui.pid

echo Waiting for UI readiness on http://localhost:3000...
set UI_RETRIES=0
:poll_ui
set /a UI_RETRIES+=1
if %UI_RETRIES% gtr 30 (
    echo [ERROR] React frontend failed to respond on http://localhost:3000 within 30 seconds.
    echo Last 20 lines of ui\ui_err.log:
    if exist ui\ui_err.log powershell -NoProfile -Command "Get-Content -Path ui\ui_err.log -Tail 20"
    goto cleanup_and_exit
)
curl -s --connect-timeout 2 --max-time 2 http://localhost:3000 >nul
if %errorlevel% neq 0 (
    ping 127.0.0.1 -n 2 >nul
    goto poll_ui
)
echo [OK] UI Console Ready (PID: %UI_PID%).

if %NO_BROWSER%==0 (
    echo Launching browser...
    start http://localhost:3000
) else (
    echo Headless/No-browser mode: skipped browser launch.
)

echo ==============================================
echo               SYSTEM READY
echo ==============================================
echo API Server:  http://localhost:8085  (PID: %API_PID%)
echo UI Console:  http://localhost:3000  (PID: %UI_PID%)
echo ==============================================
echo Press any key to stop all background processes and exit.
pause >nul

:cleanup_and_exit
echo Stopping servers...
if not "%API_PID%"=="" (
    taskkill /f /t /pid %API_PID% >nul 2>&1
)
if exist .api.pid del .api.pid >nul 2>&1

if not "%UI_PID%"=="" (
    taskkill /f /t /pid %UI_PID% >nul 2>&1
)
if exist .ui.pid del .ui.pid >nul 2>&1

:: Defensive grace period to let OS sockets release
ping 127.0.0.1 -n 2 >nul
echo Shutdown complete.
exit /b 0
