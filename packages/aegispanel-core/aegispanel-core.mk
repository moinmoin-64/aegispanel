AEGISPANEL_CORE_VERSION = 1.2.3
AEGISPANEL_CORE_SITE = $(BR2_EXTERNAL_AEGISPANEL_OS_PATH)/src/aegispanel-core
AEGISPANEL_CORE_SITE_METHOD = local
AEGISPANEL_CORE_LICENSE = MIT

AEGISPANEL_CORE_DEPENDENCIES = host-rustc openssl

define AEGISPANEL_CORE_BUILD_CMDS
	cd $(@D) && \
	PATH="$(HOST_DIR)/bin:$(PATH)" \
	CARGO_HOME="$(@D)/.cargo_home" \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(HOST_DIR)/bin/aarch64-buildroot-linux-gnu-gcc" \
	PKG_CONFIG="$(HOST_DIR)/bin/pkg-config" \
	PKG_CONFIG_SYSROOT_DIR="$(STAGING_DIR)" \
	PKG_CONFIG_LIBDIR="$(STAGING_DIR)/usr/lib/pkgconfig:$(STAGING_DIR)/usr/share/pkgconfig" \
	OPENSSL_DIR="$(STAGING_DIR)/usr" \
	OPENSSL_LIB_DIR="$(STAGING_DIR)/usr/lib" \
	OPENSSL_INCLUDE_DIR="$(STAGING_DIR)/usr/include" \
	AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_DIR="$(STAGING_DIR)/usr" \
	AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR="$(STAGING_DIR)/usr/lib" \
	AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR="$(STAGING_DIR)/usr/include" \
	cargo build --release --locked --target aarch64-unknown-linux-gnu
endef

define AEGISPANEL_CORE_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/aarch64-unknown-linux-gnu/release/aegispanel-core \
		$(TARGET_DIR)/usr/bin/aegispanel-core
endef

$(eval $(generic-package))
