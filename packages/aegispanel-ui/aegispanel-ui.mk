AEGISPANEL_UI_VERSION = 1.0.0
AEGISPANEL_UI_SITE = $(BR2_EXTERNAL_AEGISPANEL_OS_PATH)/ui
AEGISPANEL_UI_SITE_METHOD = local

define AEGISPANEL_UI_BUILD_CMDS
	if command -v npm >/dev/null 2>&1; then \
		(cd $(@D)/security && npm install && npm run build) || true; \
		(cd $(@D)/wizard && npm install && npm run build) || true; \
	fi
endef

define AEGISPANEL_UI_INSTALL_TARGET_CMDS
	mkdir -p $(TARGET_DIR)/var/www/aegispanel/security
	mkdir -p $(TARGET_DIR)/var/www/aegispanel/wizard
	if [ -d $(@D)/security/dist ]; then \
		cp -r $(@D)/security/dist/* $(TARGET_DIR)/var/www/aegispanel/security/; \
	else \
		cp -r $(@D)/security/* $(TARGET_DIR)/var/www/aegispanel/security/; \
	fi
	if [ -d $(@D)/wizard/dist ]; then \
		cp -r $(@D)/wizard/dist/* $(TARGET_DIR)/var/www/aegispanel/wizard/; \
	else \
		cp -r $(@D)/wizard/* $(TARGET_DIR)/var/www/aegispanel/wizard/; \
	fi
endef

$(eval $(generic-package))
