# PhantomCache release build (Windows PowerShell 5.1+ / PowerShell 7).
# Produces dist\phantom-cache-<version>-<target>\ and a .zip with SHA-256 checksums.
# Does NOT commit, tag, push or publish anything.
#
# Environment (optional): CARGO_TARGET_DIR, PHANTOM_BUILD_JOBS (default 1), PHANTOM_DIST_DIR.
$ErrorActionPreference = 'Stop'

$Root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $Root

$Jobs = if ($env:PHANTOM_BUILD_JOBS) { $env:PHANTOM_BUILD_JOBS } else { '1' }
$Dist = if ($env:PHANTOM_DIST_DIR) { $env:PHANTOM_DIST_DIR } else { Join-Path $Root 'dist' }

$Version = (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$Target = ((& rustc -vV) | Where-Object { $_ -like 'host:*' }).Substring(6).Trim()

# Version stamping: embedded at compile time via option_env! (see src/config.rs).
$Commit = 'unknown'
if (Get-Command git -ErrorAction SilentlyContinue) {
    # git writes to stderr when run outside a repository (for example a source copy); with
    # ErrorActionPreference=Stop that would abort the build, so relax it for these probes.
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $head = & git rev-parse HEAD 2>$null
    if ($LASTEXITCODE -eq 0 -and $head) {
        $Commit = "$head".Trim()
        if (& git status --porcelain 2>$null) { $Commit = "$Commit-dirty" }
    }
    $ErrorActionPreference = $prevEap
}
$BuildDate = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
$env:PHANTOM_BUILD_COMMIT = $Commit
$env:PHANTOM_BUILD_DATE = $BuildDate

Write-Host "Building phantom-cache-demo $Version ($Commit) for $Target"
$ErrorActionPreference = 'Continue'   # cargo reports progress on stderr
& cargo build --release --locked -j $Jobs --bin phantom-cache-demo
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
$Bin = Join-Path $TargetDir 'release\phantom-cache-demo.exe'
$Name = "phantom-cache-$Version-$Target"
$Stage = Join-Path $Dist $Name
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path (Join-Path $Stage 'docs') | Out-Null

Copy-Item $Bin $Stage
Copy-Item 'packaging\phantom.example.conf', 'packaging\phantom.production.example.conf' $Stage
if (Test-Path 'LICENSE') { Copy-Item 'LICENSE' $Stage }
Copy-Item 'docs\user' (Join-Path $Stage 'docs\user') -Recurse
Copy-Item 'docs\release' (Join-Path $Stage 'docs\release') -Recurse
Copy-Item 'Cargo.lock' (Join-Path $Stage 'Cargo.lock.snapshot')

$info = @(
    "version=$Version", "commit=$Commit", "build_date_utc=$BuildDate", "target=$Target",
    "rustc=$(& rustc --version)", "cargo=$(& cargo --version)",
    "cargo_flags=build --release --locked -j $Jobs"
)
Set-Content -Path (Join-Path $Stage 'BUILD_INFO.txt') -Value $info -Encoding ascii

$sums = Get-ChildItem -Path $Stage -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS' } | Sort-Object FullName | ForEach-Object {
    $rel = $_.FullName.Substring($Stage.Length + 1).Replace('\', '/')
    "{0}  ./{1}" -f (Get-FileHash -Algorithm SHA256 $_.FullName).Hash.ToLower(), $rel
}
Set-Content -Path (Join-Path $Stage 'SHA256SUMS') -Value $sums -Encoding ascii

$Zip = Join-Path $Dist "$Name.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path $Stage -DestinationPath $Zip
$zipHash = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLower()
Set-Content -Path "$Zip.sha256" -Value ("{0}  {1}" -f $zipHash, (Split-Path $Zip -Leaf)) -Encoding ascii

Write-Host "Release bundle: $Zip"
Write-Host "Checksum file:  $Zip.sha256"
& (Join-Path $Stage 'phantom-cache-demo.exe') --version
