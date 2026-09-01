# AegisPanel OS U-Boot Boot Script
# Supports A/B Partitioning, Boot Counter, Watchdog, and GPIO 21 Physical Recovery Jumper

# Check Physical Recovery Jumper on GPIO 21
gpio input 21
if test $? -eq 0; then
    echo "==============================================="
    echo "   RECOVERY JUMPER DETECTED! Booting Recovery  "
    echo "==============================================="
    setenv bootargs "console=tty1 console=serial0,115200 root=/dev/mmcblk0p5 rootwait panic=10"
    ext4load mmc 0:5 ${kernel_addr_r} /boot/zImage
    ext4load mmc 0:5 ${fdt_addr_r} /boot/bcm2710-rpi-zero-2-w.dtb
    bootz ${kernel_addr_r} - ${fdt_addr_r}
    exit
fi

# Set default boot slot if uninitialized
if test -z "${BOOT_SLOT}"; then
    setenv BOOT_SLOT "a"
    setenv BOOT_TRY "0"
    saveenv
fi

# Handle Try-Boot & Boot Counter Rollback
if test "${BOOT_TRY}" = "1"; then
    echo "Attempting experimental slot ${BOOT_SLOT} boot..."
    setenv BOOT_TRY "2"
    saveenv
elif test "${BOOT_TRY}" = "2"; then
    echo "Boot failed! Falling back to alternate slot..."
    if test "${BOOT_SLOT}" = "a"; then
        setenv BOOT_SLOT "b"
    else
        setenv BOOT_SLOT "a"
    fi
    setenv BOOT_TRY "0"
    saveenv
fi

# Load Partition based on active slot
if test "${BOOT_SLOT}" = "a"; then
    echo "Booting Slot A (/dev/mmcblk0p2)..."
    setenv rootpart "/dev/mmcblk0p2"
else
    echo "Booting Slot B (/dev/mmcblk0p3)..."
    setenv rootpart "/dev/mmcblk0p3"
fi

setenv bootargs "console=tty1 console=serial0,115200 root=${rootpart} rootwait rw quiet systemd.show_status=0"
fatload mmc 0:1 ${kernel_addr_r} zImage
fatload mmc 0:1 ${fdt_addr_r} bcm2710-rpi-zero-2-w.dtb
bootz ${kernel_addr_r} - ${fdt_addr_r}
