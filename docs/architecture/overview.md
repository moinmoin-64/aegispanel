# AegisPanel OS – Architecture Overview

AegisPanel OS is a custom Embedded Linux distribution engineered for Raspberry Pi Zero 2 W with a 7" 1024x600 touch display, Home Assistant Kiosk, Alarmo Security Screen, ESP32-C3 mmWave radar presence controller, and A/B update system.

## Subsystems

### 1. AegisPanel Core (`aegispanel-core`)
- Written in native **Rust**.
- Manages the deterministic system state machine.
- Communicates with Home Assistant via secure WebSocket (`wss://<ha-url>/api/websocket`).
- Listens to Alarmo `alarm_control_panel` state change events.
- Exposes a UNIX domain socket IPC server (`/run/aegispanel/ipc.sock`) with restricted 0660 permissions.
- Controls display power via DRM KMS backlight sysfs interface.
- Pings hardware watchdog timer `/dev/watchdog`.

### 2. Embedded Web Renderer (`wpewebkit` + `cog`)
- WPE WebKit provides hardware-accelerated 60 FPS HTML5/CSS3/WebSocket rendering directly on Mesa DRM KMS without X11/GTK bloat.
- Memory footprint: ~80-140MB RAM (leaving >350MB free for kernel and system).

### 3. ESP32-C3 mmWave Radar Presence Controller
- Hardware: HLK-LD2410C 24-GHz radar connected to ESP32-C3 over UART.
- Parses 24GHz radar frames for stationary and moving targets within 1.0m front envelope.
- Drives GPIO Wake line HIGH to interrupt Raspberry Pi Zero 2 W CPU from low-power sleep.

### 4. A/B Dual Partitioning & Recovery
- Dual rootfs slots (`system_a` / `system_b`).
- Signed Ed25519 payload verification.
- U-Boot tryboot counter with automatic fallback on failure.
- Physical hardware recovery jumper on GPIO 21.
