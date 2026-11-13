#!/usr/bin/env bash
# Shell script to generate reproducible Lab CA and Server Certificates for PhantomCache Demo
set -euo pipefail

OUTPUT_DIR="${1:-certs}"
SERVER_HOST="${2:-cache.lab.internal}"
SERVER_IP="${3:-127.0.0.1}"

echo "=== PhantomCache Demo Certificate Generator ==="
mkdir -p "$OUTPUT_DIR"

CA_KEY="$OUTPUT_DIR/demo-ca.key"
CA_CRT="$OUTPUT_DIR/demo-ca.crt"
SERVER_KEY="$OUTPUT_DIR/cache-server.key"
SERVER_CSR="$OUTPUT_DIR/cache-server.csr"
SERVER_CRT="$OUTPUT_DIR/cache-server.crt"
OPENSSL_CNF="$OUTPUT_DIR/server-openssl.cnf"

echo "[1/3] Generating Demo Root CA..."
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$CA_KEY" -out "$CA_CRT" -days 365 -subj "/CN=PhantomCache Demo Root CA"

echo "[2/3] Generating Server Certificate for $SERVER_HOST..."
cat > "$OPENSSL_CNF" <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = $SERVER_HOST

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $SERVER_HOST
DNS.2 = localhost
IP.1 = $SERVER_IP
IP.2 = 127.0.0.1
EOF

openssl req -newkey rsa:2048 -nodes -keyout "$SERVER_KEY" -out "$SERVER_CSR" -config "$OPENSSL_CNF"
openssl x509 -req -in "$SERVER_CSR" -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial -out "$SERVER_CRT" -days 90 -extfile "$OPENSSL_CNF" -extensions v3_req

rm -f "$SERVER_CSR" "$OPENSSL_CNF"
echo "[3/3] Certificates generated successfully in $OUTPUT_DIR!"

echo ""
echo "Multi-Device Setup Instructions:"
echo "1. On PhantomCache Node:"
echo "   export PHANTOM_TLS_CERT=\"$SERVER_CRT\""
echo "   export PHANTOM_TLS_KEY=\"$SERVER_KEY\""
echo "   export PHANTOM_BIND_ADDR=\"0.0.0.0:8443\""
echo "   ./target/release/phantom-cache-demo"
echo "2. On Requester / Client Device:"
echo "   Copy '$CA_CRT' to the client device."
echo "   curl --cacert $CA_CRT https://$SERVER_IP:8443/health"

