@echo off
setlocal enabledelayedexpansion
title PHANTOM CACHE LAUNCHER
cd /d "%~dp0"

:: Check direct CLI flags
if "%1"=="1" (
    shift
    goto run_cluster
)
if "%1"=="--cluster" (
    shift
    goto run_cluster
)
if "%1"=="2" (
    shift
    goto run_single
)
if "%1"=="--single" (
    shift
    goto run_single
)

echo ==============================================
echo               PHANTOM CACHE
echo         UNIFIED DEMONSTRATION LAUNCHER
echo ==============================================
echo.
echo Select demonstration mode:
echo   [1] Full Secure Cluster Demo (Recommended)
echo       - 3 Gateway nodes with mutual TLS and JWT auth
echo       - Real-content origin server
echo       - Control Center UI (http://localhost:3000)
echo       - Demo Hub Console (http://127.0.0.1:3201)
echo.
echo   [2] Single Gateway Standalone Demo
echo       - 1 Gateway node (http://localhost:8085)
echo       - Control Center UI (http://localhost:3000)
echo.
echo Auto-launching [1] in 5 seconds... (Press 1 or 2 to select)
echo ==============================================

choice /c 12 /t 5 /d 1 /n /m "Enter choice [1,2] (default=1): "
if errorlevel 2 goto run_single
goto run_cluster

:run_cluster
echo.
echo Launching Full Secure Cluster Demo...
call "%~dp0Start-Phantom-Cluster-Demo.bat" %*
exit /b %errorlevel%

:run_single
echo.
echo Launching Single Gateway Demo...
call "%~dp0Start-Phantom-Demo.bat" %*
exit /b %errorlevel%
