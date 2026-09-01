AEGISPANEL_UPDATER_VERSION = 1.0.0
AEGISPANEL_UPDATER_SITE = $(BR2_EXTERNAL_AEGISPANEL_OS_PATH)/packages/aegispanel-updater
AEGISPANEL_UPDATER_SITE_METHOD = local

define AEGISPANEL_UPDATER_INSTALL_TARGET_CMDS
	mkdir -p $(TARGET_DIR)/etc/rauc
	echo "[system]" > $(TARGET_DIR)/etc/rauc/system.conf
	echo "compatible=AegisPanel OS Raspberry Pi Zero 2 W" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "bootloader=uboot" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "[slot.rootfs.0]" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "device=/dev/mmcblk0p2" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "type=raw" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "bootname=A" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "[slot.rootfs.1]" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "device=/dev/mmcblk0p3" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "type=raw" >> $(TARGET_DIR)/etc/rauc/system.conf
	echo "bootname=B" >> $(TARGET_DIR)/etc/rauc/system.conf
endef

$(eval $(generic-package))
