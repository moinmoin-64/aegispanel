# AegisPanel OS – Security Model & Guidelines

## 1. Zero PIN Storage Policy
- **No Local Storage:** AegisPanel OS NEVER saves, logs, caches, hashes, or validates the Alarmo security PIN.
- **Transient RAM Lifetime:** PIN entered on the touch UI is held strictly in volatile RAM during packet construction.
- **Memory Zeroization:** The Rust `aegispanel-core` daemon utilizes the `zeroize` crate to securely overwrite the PIN buffer in RAM immediately after sending the disarm WebSocket payload to Home Assistant.
- **No PIN in Logs:** All log messages strictly redact or exclude security credentials.

## 2. Secrets Encryption & File Permissions
- Long-Lived Access Tokens (LLAT) for Home Assistant and Wi-Fi PSK are saved in `/etc/aegispanel/secrets.json`.
- Strict file permission `0600` (Read/Write for root/aegispanel only).
- Home Assistant Access Token is NEVER exposed to JavaScript in browser bundles or UI HTML.

## 3. Network & Transport Security
- TLS enforcement (`https://` and `wss://`).
- Local IPC socket restricted to `0660` permissions.
- Minimal Linux services running (no open SSH root ports by default, no unauthenticated HTTP listeners).

## 4. OTA Firmware Integrity
- Ed25519 digital signature verification on all OTA payloads.
- SHA256 checksum validation.
- Automatic rollback to alternate slot if post-boot health check fails or watchdog trips.
