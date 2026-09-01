#!/bin/bash
# AegisPanel OS – SD Card Image Creator
# Assembles the final bootable sdcard.img for Raspberry Pi Zero 2 W
set -e

IMAGES_DIR="/home/oliver/dev/panel/buildroot/output/images"
OUTPUT_IMG="${IMAGES_DIR}/sdcard.img"
WORK_DIR="/home/oliver/dev/panel/buildroot/output/build/sdcard-work"
FIRMWARE_DIR="${WORK_DIR}/firmware"
BOOT_DIR="${WORK_DIR}/boot"

# Image layout (in MiB)
BOOT_SIZE=128
ROOT_SIZE=2600
TOTAL_SIZE=$((BOOT_SIZE + ROOT_SIZE + 4))  # +4 MiB for alignment/GPT

echo "=================================================="
echo "   AegisPanel OS – SD Card Image Builder          "
echo "=================================================="

# ── 1. Sanity checks ─────────────────────────────────
if [ ! -f "${IMAGES_DIR}/Image" ]; then
    echo "ERROR: Kernel image not found at ${IMAGES_DIR}/Image"
    exit 1
fi
if [ ! -f "${IMAGES_DIR}/rootfs.ext2" ]; then
    echo "ERROR: Root filesystem not found at ${IMAGES_DIR}/rootfs.ext2"
    exit 1
fi

# ── 2. Download RPi Firmware ──────────────────────────
echo "Downloading Raspberry Pi firmware files..."
mkdir -p "${FIRMWARE_DIR}" "${BOOT_DIR}"

RPI_FIRMWARE_BASE="https://github.com/raspberrypi/firmware/raw/stable/boot"
for f in bootcode.bin start.elf start_cd.elf fixup.dat fixup_cd.dat; do
    if [ ! -f "${FIRMWARE_DIR}/${f}" ]; then
        echo "  Downloading ${f}..."
        wget -q -O "${FIRMWARE_DIR}/${f}" "${RPI_FIRMWARE_BASE}/${f}" || \
            wget -q -O "${FIRMWARE_DIR}/${f}" "https://raw.githubusercontent.com/raspberrypi/firmware/stable/boot/${f}"
    fi
done

# ── 3. Create config.txt ──────────────────────────────
cat > "${BOOT_DIR}/config.txt" << 'EOF'
# AegisPanel OS – Raspberry Pi Zero 2 W Boot Configuration
arm_64bit=1
kernel=Image
initramfs initrd.img followkernel

# Display: 7" 1024x600 Touchscreen
hdmi_group=2
hdmi_mode=87
hdmi_cvt=1024 600 60 6 0 0 0
hdmi_drive=2
display_rotate=0

# GPU memory split (minimal – no 3D workload)
gpu_mem=128

# UART for debug console
enable_uart=1
dtparam=uart0=on

# Disable activity LED (panel aesthetics)
dtparam=act_led_trigger=none

# Power management
dtparam=watchdog=on

# Overlay: A/B boot slot (U-Boot handles this)
# Uncomment if using U-Boot:
# kernel=u-boot.bin
EOF

# ── 4. Create cmdline.txt ─────────────────────────────
cat > "${BOOT_DIR}/cmdline.txt" << 'EOF'
console=serial0,115200 console=tty1 root=/dev/mmcblk0p2 rootfstype=ext4 elevator=deadline fsck.repair=yes rootwait quiet splash loglevel=3
EOF

# ── 5. Assemble boot partition contents ───────────────
cp "${FIRMWARE_DIR}"/{bootcode.bin,start.elf,start_cd.elf,fixup.dat,fixup_cd.dat} "${BOOT_DIR}/"
cp "${IMAGES_DIR}/Image" "${BOOT_DIR}/"
cp "${IMAGES_DIR}/bcm2837-rpi-zero-2-w.dtb" "${BOOT_DIR}/"

# Device Tree overlays directory
mkdir -p "${BOOT_DIR}/overlays"

echo "Boot partition contents:"
ls -lh "${BOOT_DIR}/"

# ── 6. Create raw disk image ──────────────────────────
echo ""
echo "Creating ${TOTAL_SIZE} MiB disk image..."
rm -f "${OUTPUT_IMG}"
dd if=/dev/zero of="${OUTPUT_IMG}" bs=1M count="${TOTAL_SIZE}" status=progress

# ── 7. Partition table (MBR) ──────────────────────────
echo "Partitioning image..."
parted -s "${OUTPUT_IMG}" \
    mklabel msdos \
    mkpart primary fat32 4MiB "$((4 + BOOT_SIZE))MiB" \
    mkpart primary ext4 "$((4 + BOOT_SIZE))MiB" "100%" \
    set 1 boot on

# ── 8. Mount and format partitions via loop device ───
LOOP_DEV=$(losetup --find --show --partscan "${OUTPUT_IMG}")
echo "Loop device: ${LOOP_DEV}"

sleep 1  # Wait for kernel to scan partitions

# Format boot partition as FAT32
mkfs.vfat -F 32 -n "AEGISBOOT" "${LOOP_DEV}p1"

# Copy rootfs to second partition (already ext4 formatted)
echo "Copying root filesystem (this may take a few minutes)..."
dd if="${IMAGES_DIR}/rootfs.ext2" of="${LOOP_DEV}p2" bs=4M status=progress
sync

# ── 9. Mount boot partition and copy files ────────────
BOOT_MOUNT="${WORK_DIR}/boot_mount"
mkdir -p "${BOOT_MOUNT}"
mount "${LOOP_DEV}p1" "${BOOT_MOUNT}"

echo "Copying boot files..."
cp -r "${BOOT_DIR}/"* "${BOOT_MOUNT}/"
sync

umount "${BOOT_MOUNT}"
losetup -d "${LOOP_DEV}"

# ── 10. Done ──────────────────────────────────────────
echo ""
echo "=================================================="
echo "   SUCCESS! SD Card Image Created                 "
echo "=================================================="
echo ""
echo "  Image: ${OUTPUT_IMG}"
echo "  Size:  $(du -h "${OUTPUT_IMG}" | cut -f1)"
echo ""
echo "  Flash to SD card with:"
echo "  sudo dd if=${OUTPUT_IMG} of=/dev/sdX bs=4M status=progress conv=fsync"
echo "  (Replace /dev/sdX with your SD card device)"
echo ""
