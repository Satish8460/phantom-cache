@echo off
setlocal
title PHANTOM CACHE - SECURE CLUSTER DEMO
cd /d "%~dp0"
rem Git for Windows ships openssl (used once to create the demo certificates). Its folders are added at the END of PATH,
rem only for this window, so an openssl you installed yourself still wins and nothing global is changed.
set "PATH=%PATH%;%ProgramFiles%\Git\usr\bin;%ProgramFiles%\Git\mingw64\bin;%ProgramFiles(x86)%\Git\usr\bin;%LOCALAPPDATA%\Programs\Git\usr\bin;%ProgramFiles%\OpenSSL-Win64\bin;%USERPROFILE%\.cargo\bin"

echo ==============================================
echo        PHANTOM CACHE  -  SECURE CLUSTER DEMO
echo ==============================================
echo   3 gateways (mutual TLS, bearer tokens), real-content origin,
echo   demo console, Control Center. One command, one window.
echo.

set NO_BROWSER=0
for %%a in (%*) do (
    if "%%a"=="--no-browser" set NO_BROWSER=1
    if "%%a"=="--headless" set NO_BROWSER=1
)

set "WORK=%LOCALAPPDATA%\PhantomCacheDemo"
if not exist "%WORK%" mkdir "%WORK%"

echo [1/4] Checking prerequisites, ports, binary and UI dependencies...
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\demo_preflight.ps1"
if errorlevel 1 (
    echo.
    echo Startup stopped. Fix the problem above and run this file again.
    pause
    exit /b 1
)

echo.
echo [2/4] Starting the Control Center on http://localhost:3000 ...
powershell -NoProfile -Command "$p = Start-Process -FilePath 'npm.cmd' -ArgumentList 'run dev -- --port 3000 --strictPort' -WorkingDirectory 'ui' -PassThru -WindowStyle Hidden -RedirectStandardOutput '%WORK%\ui.log' -RedirectStandardError '%WORK%\ui_err.log'; Set-Content -Path '%WORK%\ui.pid' -Value $p.Id"
set UI_PID=
if exist "%WORK%\ui.pid" set /p UI_PID=<"%WORK%\ui.pid"

set UI_TRIES=0
:wait_ui
set /a UI_TRIES+=1
curl -s --connect-timeout 2 --max-time 2 http://localhost:3000 >nul 2>&1
if errorlevel 1 (
    if %UI_TRIES% lss 30 (
        ping 127.0.0.1 -n 2 >nul
        goto wait_ui
    )
    echo [ERROR] The Control Center did not answer within 30 seconds. See %WORK%\ui_err.log
    if not "%UI_PID%"=="" taskkill /f /t /pid %UI_PID% >nul 2>&1
    pause
    exit /b 1
)
echo [OK] Control Center is up.

echo.
echo [3/4] Starting the secure cluster and the demo console (about 20 seconds)...
if "%NO_BROWSER%"=="0" (
    start "" /min powershell -NoProfile -WindowStyle Hidden -Command "Start-Sleep -Seconds 28; Start-Process 'http://127.0.0.1:3201'"
)
echo.
echo ----------------------------------------------
echo   Demo console:    http://127.0.0.1:3201
echo   Control Center:  http://localhost:3000   (sign in with a token from the console)
echo   To stop: press Ctrl+C once here; if asked "Terminate batch job", answer N.
echo ----------------------------------------------
echo.
node scripts\demo_flow.mjs --exe "%CD%\target\release\phantom-cache-demo.exe" --dir "%WORK%\cluster" --console --ui-base http://localhost:3000

echo.
echo [4/4] Stopping everything...
node scripts\cluster_launcher.mjs down --dir "%WORK%\cluster" >nul 2>&1
if not "%UI_PID%"=="" taskkill /f /t /pid %UI_PID% >nul 2>&1
if exist "%WORK%\ui.pid" del "%WORK%\ui.pid" >nul 2>&1
echo Shutdown complete.
exit /b 0
