# Preflight for Start-Phantom-Cluster-Demo.bat: checks prerequisites, ports, the release binary (builds it when missing or older
# than the sources) and the UI dependencies. Exit code 0 = ready, 1 = a problem was printed. Reads and builds only; it never deletes anything.
param([string]$Root = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-Location $Root
$env:Path += ";$env:USERPROFILE\.cargo\bin"

function Fail($m) { Write-Host "[ERROR] $m" -ForegroundColor Red; exit 1 }

if (-not (Get-Command node -ErrorAction SilentlyContinue)) { Fail 'Node.js is not installed or not in PATH.' }
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) { Fail 'npm is not installed or not in PATH.' }
# openssl is used once to create the demo certificates: look in the usual places (Git for Windows ships it) and add the folder to PATH for this run only.
foreach ($d in @("$env:ProgramFiles\Git\usr\bin", "$env:ProgramFiles\Git\mingw64\bin", "${env:ProgramFiles(x86)}\Git\usr\bin", "$env:LOCALAPPDATA\Programs\Git\usr\bin", "$env:ProgramFiles\OpenSSL-Win64\bin")) {
  if (-not (Get-Command openssl -ErrorAction SilentlyContinue) -and (Test-Path (Join-Path $d 'openssl.exe'))) { $env:Path += ";$d" }
}
if (-not (Get-Command openssl -ErrorAction SilentlyContinue)) { Fail 'openssl is not in PATH (the launcher needs it once, to create the demo certificates). Git for Windows ships it: add its usr\bin folder to PATH.' }

# Ports used by the demo: UI 3000, console 3201, origin 19099, gateways 18185-18187, cluster listeners 19501-19503.
$ports = 3000, 3201, 19099, 18185, 18186, 18187, 19501, 19502, 19503
$busy = @()
foreach ($p in $ports) {
  $c = Get-NetTCPConnection -LocalPort $p -State Listen -ErrorAction SilentlyContinue
  if ($c) { $busy += "$p (process $($c[0].OwningProcess))" }
}
if ($busy.Count -gt 0) { Fail ("These ports are already in use: " + ($busy -join ', ') + '. Close the other PhantomCache demo (or run: node scripts\cluster_launcher.mjs down --dir <work dir>) and try again.') }

# Release binary: build when missing or older than the sources.
$bin = Join-Path $Root 'target\release\phantom-cache-demo.exe'
$needBuild = -not (Test-Path $bin)
if (-not $needBuild) {
  $t = (Get-Item $bin).LastWriteTime
  $newer = Get-ChildItem -Path (Join-Path $Root 'src'), (Join-Path $Root 'Cargo.toml'), (Join-Path $Root 'Cargo.lock') -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -gt $t } | Select-Object -First 1
  if ($newer) { $needBuild = $true }
}
if ($needBuild) {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Fail 'The release binary is missing or out of date and cargo is not installed.' }
  Write-Host '[INFO] Building the release gateway (first run takes a few minutes)...'
  $env:CARGO_INCREMENTAL = '0'
  & cargo build --release --bin phantom-cache-demo -j 1 --locked
  if ($LASTEXITCODE -ne 0) { Fail 'The release build failed.' }
}
Write-Host "[OK] Gateway binary: $bin"

# UI dependencies.
if (-not (Test-Path (Join-Path $Root 'ui\node_modules'))) {
  Write-Host '[INFO] Installing UI dependencies (npm ci)...'
  Push-Location (Join-Path $Root 'ui')
  & npm.cmd ci
  $code = $LASTEXITCODE
  Pop-Location
  if ($code -ne 0) { Fail 'npm ci failed.' }
}
Write-Host '[OK] UI dependencies present.'
exit 0
