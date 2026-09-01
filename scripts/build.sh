#!/bin/bash
# AegisPanel OS Monorepo Build Orchestration Script
set -e

echo "=================================================="
echo "   AegisPanel OS Build System (Developer PC)      "
echo "=================================================="

# Buildroot refuses to build as root without this flag
if [ "$(id -u)" = "0" ]; then
    echo "WARNING: Running as root – setting FORCE_UNSAFE_CONFIGURE=1"
    export FORCE_UNSAFE_CONFIGURE=1
fi

BUILDROOT_DIR="buildroot"
EXTERNAL_PATH="$(pwd)"

if [ ! -d "$BUILDROOT_DIR" ]; then
    echo "Cloning Buildroot LTS repository..."
    git clone --depth 1 -b 2024.02.x https://github.com/buildroot/buildroot.git "$BUILDROOT_DIR"
fi

echo "Configuring AegisPanel OS Buildroot environment..."
cd "$BUILDROOT_DIR"

# Clean stale config cache and linux dotconfig stamps if defconfig updated
rm -rf output/build/buildroot-config output/.config output/build/linux-*/.stamp_dotconfig

# Clean cached Cargo Edition 2024 incompatible crates from root/user cargo registry
find ~/.cargo/registry/src/ -name "*chacha20-0.10.1*" -exec rm -rf {} + 2>/dev/null || true
find ~/.cargo/registry/src/ -name "*base64ct-1.8.3*" -exec rm -rf {} + 2>/dev/null || true

# Ensure Cargo.lock exists for local Rust packages
if [ -f "$EXTERNAL_PATH/buildroot/output/host/bin/cargo" ]; then
    (cd "$EXTERNAL_PATH/src/aegispanel-core" && "$EXTERNAL_PATH/buildroot/output/host/bin/cargo" generate-lockfile) || true
    (cd "$EXTERNAL_PATH/src/recovery" && "$EXTERNAL_PATH/buildroot/output/host/bin/cargo" generate-lockfile) || true
fi

make BR2_EXTERNAL="$EXTERNAL_PATH" aegispanel_rpi_zero2w_defconfig

# Patch the overlay path to use the absolute external path (defconfig uses relative paths)
CONFIG_FILE=".config"
if [ ! -f "$CONFIG_FILE" ] && [ -f "output/.config" ]; then
    CONFIG_FILE="output/.config"
fi
sed -i "s|BR2_ROOTFS_OVERLAY=\".*\"|BR2_ROOTFS_OVERLAY=\"$EXTERNAL_PATH/board/raspberrypi/zero2w/overlays/aegispanel-overlay\"|" "$CONFIG_FILE"

echo "Starting cross-compilation build (this may take a while)..."
make -j$(nproc)

echo "=================================================="
echo "   Build Finished Successfully!                   "
echo "   Image: $BUILDROOT_DIR/output/images/sdcard.img "
echo "=================================================="
