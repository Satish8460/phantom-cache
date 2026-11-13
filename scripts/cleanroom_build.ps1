# Clean-room build and smoke test (PowerShell). New in M7.
# Copies the working tree (without target, .git, node_modules, dist, logs) into a fresh temp
# directory, builds a release bundle there with a FRESH CARGO_TARGET_DIR (-j 1, --locked),
# verifies checksums and runs the packaged smoke test against the extracted-in-place bundle.
#
# "Clean environment" here means: fresh source copy + fresh target dir on THIS machine. It is NOT
# a clean-machine test (same OS image, same installed toolchain and cargo registry cache).
#
# Usage: scripts\cleanroom_build.ps1 -WorkDir D:\scratch\phantom-cleanroom [-Port 18090]
param([Parameter(Mandatory = $true)][string]$WorkDir, [int]$Port = 18090, [string]$TargetDir = "")
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$src = Join-Path $WorkDir 'src-copy'
$tgt = if ($TargetDir) { $TargetDir } else { Join-Path $WorkDir "target" }
$dist = Join-Path $WorkDir 'dist'
if (Test-Path $WorkDir) { throw "WorkDir '$WorkDir' already exists; choose a new directory" }
New-Item -ItemType Directory -Path $src, $dist | Out-Null
$sw = [Diagnostics.Stopwatch]::StartNew()

# Exclude by FULL path: a bare name like "dist" would also drop srcdist (the distributed-cache module).
$xd = @("target", ".git", "dist", "repro-logs", "bench-results", "lab-results") | ForEach-Object { Join-Path $root $_ }
$xd += (Join-Path $root "ui
ode_modules")
robocopy $root $src /E /XD $xd /XF *.log transcript_copy.jsonl /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed: $LASTEXITCODE" }

$env:CARGO_TARGET_DIR = $tgt
$env:PHANTOM_DIST_DIR = $dist
$env:PHANTOM_BUILD_JOBS = '1'
Push-Location $src
try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File packaging\build-release.ps1
    if ($LASTEXITCODE -ne 0) { throw "build-release failed ($LASTEXITCODE)" }
} finally { Pop-Location }

$bundle = Get-ChildItem $dist -Directory | Select-Object -First 1
Push-Location $bundle.FullName
$bad = 0
foreach ($line in Get-Content SHA256SUMS) {
    $h, $p = $line -split '\s+', 2
    $p = $p.TrimStart('.', '/')
    if ((Get-FileHash -Algorithm SHA256 $p).Hash.ToLower() -ne $h) { Write-Host "CHECKSUM MISMATCH: $p"; $bad++ }
}
Pop-Location
if ($bad) { throw "$bad checksum mismatches" }
Write-Host 'SHA256SUMS verified'

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $src 'packaging\smoke-test.ps1') -BundleDir $bundle.FullName -Port $Port
if ($LASTEXITCODE -ne 0) { throw "smoke test failed ($LASTEXITCODE)" }
$sw.Stop()
Write-Host ("cleanroom build + smoke test passed in {0:N0} s; bundle: {1}" -f $sw.Elapsed.TotalSeconds, $bundle.FullName)
