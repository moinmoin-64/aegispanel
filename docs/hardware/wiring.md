# AegisPanel OS – Hardware Wiring & Power Diagram

## Component List
1. Main SBC: **Raspberry Pi Zero 2 W** (ARM Cortex-A53, 512MB RAM)
2. Display: **7" Touchscreen** (1024x600, HDMI Video, USB Touch Controller)
3. MCU Controller: **ESP32-C3** SuperMini / DevBoard
4. mmWave Radar: **HLK-LD2410C** (24-GHz radar module)
5. Power Supply: **5V / 3A DC PSU**

## Wiring Schematic

```
                          ┌──────────────────────────┐
                          │ 5V / 3A Power Distribution│
                          └─────────────┬────────────┘
                                        │
           ┌────────────────────────────┼────────────────────────────┐
           ▼                            ▼                            ▼
┌──────────────────────┐    ┌──────────────────────┐    ┌──────────────────────┐
│ Raspberry Pi Zero 2W │    │ 7" Touchscreen Display│   │   ESP32-C3 Controller│
└──────────┬───────────┘    └──────────────────────┘    └──────────┬───────────┘
           │                                                       │
           │ GPIO 4 (Wake In) <────────────────────────────────────┤ GPIO 4 (Wake Out)
           │ GPIO 21 (Recovery Jumper to GND)                      │
           │                                                       │
           │                                                       │ UART2 (TX:21, RX:20)
           │                                                       ▼
           │                                            ┌──────────────────────┐
           │                                            │ HLK-LD2410C Radar    │
           │                                            └──────────────────────┘
```

## GPIO Pinout Table

| Source Device | Pin Name | Target Device | Target Pin | Function |
| :--- | :--- | :--- | :--- | :--- |
| **ESP32-C3** | GPIO 4 | RPi Zero 2 W | GPIO 4 (Pin 7) | Active HIGH Presence Wake Signal |
| **RPi Zero 2 W**| GPIO 21 | Header / Jumper | GND (Pin 39) | Active LOW Recovery Jumper |
| **ESP32-C3** | GPIO 21 (TX) | HLK-LD2410C | RX | Radar UART (256000 baud) |
| **ESP32-C3** | GPIO 20 (RX) | HLK-LD2410C | TX | Radar UART (256000 baud) |
