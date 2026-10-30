@echo off
title PHANTOM DEMO LAUNCHER
echo ==============================================
echo               PHANTOM CACHE
echo            INITIALIZING SYSTEM
echo ==============================================

set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"

echo Checking environment prerequisites...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js is not installed.
    pause
    exit /b 1
)

where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] npm is not installed.
    pause
    exit /b 1
)

echo Verifying port availability...
netstat -ano | findstr :8085 >nul
if %errorlevel%==0 (
    echo [WARNING] Port 8085 is already occupied.
    echo Please ensure no other application is using port 8085.
    pause
    exit /b 1
)

netstat -ano | findstr :3000 >nul
if %errorlevel%==0 (
    echo [WARNING] Port 3000 is already occupied.
    echo Please ensure no other application is using port 3000.
    pause
    exit /b 1
)

echo Starting Axum API server...
if not exist target\release\phantom-cache-demo.exe (
    echo Building release binary...
    set CARGO_INCREMENTAL=0
    cargo build --release > build.log 2>&1
)
start /b target\release\phantom-cache-demo.exe > api.log 2>&1
for /f "tokens=2" %%i in ('tasklist /nh /fi "imagename eq phantom-cache-demo.exe"') do set API_PID=%%i

echo Waiting for API readiness...
:poll_health
curl -s http://localhost:8085/health | findstr OK >nul
if %errorlevel% neq 0 (
    ping 127.0.0.1 -n 2 >nul
    goto poll_health
)
echo [OK] API Server Ready.

echo Starting React frontend...
cd ui
start /b cmd /c "npm run dev -- --port 3000 > ui.log 2>&1"
cd ..

echo Waiting for UI readiness...
:poll_ui
curl -s http://localhost:3000 >nul
if %errorlevel% neq 0 (
    ping 127.0.0.1 -n 2 >nul
    goto poll_ui
)
echo [OK] UI Console Ready.

echo Launching browser...
start http://localhost:3000

echo ==============================================
echo               SYSTEM READY
echo ==============================================
echo Press any key to stop all background processes and exit.
pause >nul

echo Stopping servers...
if not "%API_PID%"=="" (
    taskkill /f /pid %API_PID% >nul 2>&1
)
echo Shutdown complete.
exit /b 0
