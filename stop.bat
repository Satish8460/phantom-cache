@echo off
setlocal enabledelayedexpansion
title STOP PHANTOM CACHE
cd /d "%~dp0"

echo ==============================================
echo        PHANTOM CACHE - STOPPING DEMO
echo ==============================================

set "WORK=%LOCALAPPDATA%\PhantomCacheDemo"

if exist "%WORK%\cluster" (
    node scripts\cluster_launcher.mjs down --dir "%WORK%\cluster" >nul 2>&1
)

:: Cleanly terminate lingering gateway and tunnel daemons
taskkill /f /im phantom-cache-demo.exe >nul 2>&1
taskkill /f /im cloudflared.exe >nul 2>&1

:: Stop UI process if pid file exists
if exist "%WORK%\ui.pid" (
    set /p UI_PID=<"%WORK%\ui.pid"
    if not "!UI_PID!"=="" taskkill /f /t /pid !UI_PID! >nul 2>&1
    del "%WORK%\ui.pid" >nul 2>&1
)

echo [OK] All PhantomCache processes have been stopped and ports (18185-18187, 19099, 3201, 3000) are free.
echo You can now run start.bat cleanly.
