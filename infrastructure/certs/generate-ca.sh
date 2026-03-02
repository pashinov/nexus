#!/bin/bash
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ -f "$DIR/ca.pem" ]]; then
    echo "CA already exists: $DIR/ca.pem"
    exit 0
fi

echo "Generating CA key and certificate..."

openssl genrsa -out "$DIR/ca.key" 4096

openssl req -new -x509 -days 365 \
    -key "$DIR/ca.key" \
    -out "$DIR/ca.pem" \
    -subj "/CN=Nexus CA/O=Nexus"

echo "Generating EMQX server certificate..."

DOMAIN="${SERVER_DOMAIN:-mqtt.apashinov.com}"

openssl genrsa -out "$DIR/server.key" 2048

openssl req -new \
    -key "$DIR/server.key" \
    -out "$DIR/server.csr" \
    -subj "/CN=$DOMAIN/O=Nexus"

openssl x509 -req -days 36500 \
    -in "$DIR/server.csr" \
    -CA "$DIR/ca.pem" \
    -CAkey "$DIR/ca.key" \
    -CAcreateserial \
    -extfile <(printf "subjectAltName=DNS:%s" "$DOMAIN") \
    -out "$DIR/server.pem"

echo "Generating client certificate..."

openssl genrsa -out "$DIR/client.key" 2048

openssl req -new \
    -key "$DIR/client.key" \
    -out "$DIR/client.csr" \
    -subj "/CN=nexus-device/O=Nexus"

openssl x509 -req -days 3650 \
    -in "$DIR/client.csr" \
    -CA "$DIR/ca.pem" \
    -CAkey "$DIR/ca.key" \
    -CAcreateserial \
    -out "$DIR/client.pem"

rm -f "$DIR"/*.csr "$DIR"/*.srl

echo ""
echo "Done:"
echo "  CA:     ca.pem"
echo "  Server: server.pem + server.key  (deploy to EMQX)"
echo "  Client: client.pem + client.key  (deploy to devices)"
echo ""
echo "Keep *.key secret!"
