#!/bin/bash
# AegisPanel OS Ed25519 Key Generation Script for OTA Updates
set -e

KEY_DIR="board/raspberrypi/zero2w/overlays/aegispanel-overlay/etc/aegispanel"
mkdir -p "$KEY_DIR"

echo "Generating Ed25519 keypair for OTA update verification..."

openssl genpkey -algorithm ed25519 -out ota_privkey.pem
openssl pkey -in ota_privkey.pem -pubout -outform DER -out "$KEY_DIR/ota_pubkey.bin"

echo "Ed25519 Keys Generated:"
echo " Private Key (Keep secret): ota_privkey.pem"
echo " Public Key (Installed in OS): $KEY_DIR/ota_pubkey.bin"
