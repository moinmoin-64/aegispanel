# AegisPanel OS

**AegisPanel OS** is a production-grade, highly secure, minimal Embedded Linux operating system built with **Buildroot** for the **Raspberry Pi Zero 2 W** (512 MB RAM) with a 7" Touchdisplay (1024×600), an **ESP32-C3** low-power mmWave presence controller, and **Home Assistant / Alarmo** security integration.

---

## 🌟 Key Features

* 🚀 **Resource Efficient:** Uses **WPE WebKit + Cog** renderer (~80–140 MB RAM usage), leaving >350 MB free RAM.
* 🛡️ **Zero PIN Storage Policy:** Security PIN is transmitted directly to Alarmo over TLS and zeroized in volatile RAM using Rust's `zeroize`.
* 📡 **mmWave Presence Sensing:** ESP32-C3 continuously parses 24GHz HLK-LD2410C radar data (1.0m front envelope) to wake display via GPIO.
* 🌙 **Smart Night Mode:** Display powers OFF between 22:00–06:30; Alarmo `armed_*` state changes override night mode instantly.
* 🔄 **Atomic A/B OTA Updates:** Dual partition slots with Ed25519 digital signatures, SHA256 hashes, and U-Boot automatic rollback.
* 🚨 **Emergency Recovery System:** Physical GPIO 21 hardware jumper triggers dedicated minimal recovery OS.
* ⚡ **Deterministic State Machine:** Built in native Rust (`aegispanel-core`).

---

## 📁 Repository Structure

```
aegispanel-os/
├── board/               # RPi Zero 2 W config, U-Boot scripts, overlays, systemd services
├── configs/             # Buildroot defconfig for Raspberry Pi Zero 2 W
├── docs/                # Architecture, Hardware Wiring & Security Documentation
├── esp32/               # ESP-IDF C++ Firmware for ESP32-C3 mmWave Presence Controller
├── packages/            # Custom Buildroot external package recipes
├── scripts/             # Build, SD Flashing, and Ed25519 Key Generation scripts
├── src/
│   ├── aegispanel-core/ # Native Rust Core Daemon (State Machine, WSS Client, IPC, Power, OTA)
│   └── recovery/        # Minimal Recovery Boot Subsystem
└── ui/
    ├── security/        # Dark-mode Touch Keypad Security Screen (TypeScript / Vite)
    └── wizard/          # First-Boot Setup Wizard UI (TypeScript / Vite)
```

---

## 🛠️ Quick Start & Build Instructions

### 1. Generate OTA Keys
```bash
./scripts/generate_keys.sh
```

### 2. Build OS Image (Developer PC)
```bash
./scripts/build.sh
```

### 3. Flash MicroSD Card
```bash
./scripts/flash_sd.sh /dev/sdX
```

### 4. Build ESP32-C3 Firmware
```bash
cd esp32/mmwave-controller
idf.py set-target esp32c3
idf.py build flash monitor
```

---

## 📄 License
MIT License. Developed for AegisPanel OS Embedded Linux Systems.
