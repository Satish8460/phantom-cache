# PowerShell script to generate reproducible Lab CA and Server Certificates for PhantomCache Demo
param (
    [string]$OutputDir = "certs",
    [string]$ServerHost = "cache.lab.internal",
    [string]$ServerIP = "127.0.0.1"
)

Write-Host "=== PhantomCache Demo Certificate Generator ===" -ForegroundColor Cyan

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

$caKey = Join-Path $OutputDir "demo-ca.key"
$caCrt = Join-Path $OutputDir "demo-ca.crt"
$serverKey = Join-Path $OutputDir "cache-server.key"
$serverCsr = Join-Path $OutputDir "cache-server.csr"
$serverCrt = Join-Path $OutputDir "cache-server.crt"
$opensslCnf = Join-Path $OutputDir "server-openssl.cnf"

# 1. Check for OpenSSL
$hasOpenSSL = Get-Command openssl -ErrorAction SilentlyContinue

if ($hasOpenSSL) {
    Write-Host "[1/3] Generating Demo Root CA with OpenSSL..." -ForegroundColor Green
    openssl req -x509 -newkey rsa:2048 -nodes -keyout $caKey -out $caCrt -days 365 -subj "/CN=PhantomCache Demo Root CA" 2>$null

    Write-Host "[2/3] Generating Server Certificate for $ServerHost..." -ForegroundColor Green
    @"
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = $ServerHost

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $ServerHost
DNS.2 = localhost
IP.1 = $ServerIP
IP.2 = 127.0.0.1
"@ | Set-Content -Path $opensslCnf -Encoding ascii

    openssl req -newkey rsa:2048 -nodes -keyout $serverKey -out $serverCsr -config $opensslCnf 2>$null
    openssl x509 -req -in $serverCsr -CA $caCrt -CAkey $caKey -CAcreateserial -out $serverCrt -days 90 -extfile $opensslCnf -extensions v3_req 2>$null

    Remove-Item $serverCsr -ErrorAction SilentlyContinue
    Remove-Item $opensslCnf -ErrorAction SilentlyContinue
    Write-Host "[3/3] Certificates generated successfully in $OutputDir!" -ForegroundColor Green
} else {
    Write-Host "[INFO] OpenSSL not found in PATH. Using automated test runner generation via 'rcgen' crate." -ForegroundColor Yellow
}

Write-Host "`nUsage Instructions for Multi-Device Demonstration:" -ForegroundColor Cyan
Write-Host "1. On PhantomCache Node:"
Write-Host "   Set environment variables:"
Write-Host "   `$env:PHANTOM_TLS_CERT = '$serverCrt'"
Write-Host "   `$env:PHANTOM_TLS_KEY  = '$serverKey'"
Write-Host "   `$env:PHANTOM_BIND_ADDR = '0.0.0.0:8443'"
Write-Host "2. On Requester / Client Devices:"
Write-Host "   Copy '$caCrt' to the client device."
Write-Host "   Execute with explicit CA trust (NEVER use --insecure):"
Write-Host "   curl --cacert $caCrt https://${ServerIP}:8443/health"

