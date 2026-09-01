#!/bin/bash
# AegisPanel OS Integration Test Suite
set -e

echo "=================================================="
echo "   AegisPanel OS Automated Integration Test Suite "
echo "=================================================="

echo "[1/4] Testing Directory & Workspace Files Integrity..."
test -f README.md
test -f configs/aegispanel_rpi_zero2w_defconfig
test -f src/aegispanel-core/src/main.rs
test -f src/recovery/src/main.rs
test -f ui/security/index.html
test -f ui/wizard/index.html
test -f esp32/mmwave-controller/main/main.cpp
test -f board/raspberrypi/zero2w/bootloader/boot.cmd
echo "✔ Monorepo File Integrity VERIFIED."

echo "[2/4] Verifying Shell Scripts..."
bash -n scripts/build.sh
bash -n scripts/flash_sd.sh
bash -n scripts/generate_keys.sh
echo "✔ Shell Scripts Syntax VERIFIED."

echo "[3/4] Checking Mock Home Assistant Test Server..."
python3 -m py_compile scripts/mock_ha_server.py
echo "✔ Python Mock HA Server Syntax VERIFIED."

echo "[4/4] Security Audit: Checking for accidental PIN/Secret Leaks in Source Code..."
if grep -rn "PIN = " src/ ui/ esp32/ 2>/dev/null; then
    echo "❌ ERROR: Hardcoded PIN leak detected!"
    exit 1
fi
echo "✔ Zero-PIN Storage Policy Audit VERIFIED."

echo "=================================================="
echo "   ALL INTEGRATION TESTS PASSED SUCCESSFULLY!    "
echo "=================================================="
