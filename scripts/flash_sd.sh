#!/bin/bash
# AegisPanel OS SD Card Flashing Script
set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 /dev/sdX"
    exit 1
fi

TARGET_DEV="$1"
IMG_PATH="buildroot/output/images/sdcard.img"

if [ ! -f "$IMG_PATH" ]; then
    echo "Error: $IMG_PATH does not exist. Run scripts/build.sh first!"
    exit 1
fi

echo "WARNING: Writing to $TARGET_DEV will ERASE ALL DATA!"
read -p "Are you sure you want to proceed? (y/N): " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Aborted."
    exit 0
fi

echo "Flashing AegisPanel OS image..."
sudo dd if="$IMG_PATH" of="$TARGET_DEV" bs=4M status=progress conv=fsync
echo "Flashing completed successfully!"
