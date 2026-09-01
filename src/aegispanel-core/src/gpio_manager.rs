#![allow(dead_code)]

use tokio::sync::mpsc;
use tracing::{info, warn};

pub const DEFAULT_ESP32_WAKE_PIN: u8 = 4;
pub const DEFAULT_RECOVERY_JUMPER_PIN: u8 = 17;

pub enum GpioEvent {
    PresenceWakeTriggered,
    PresenceGone,
    RecoveryJumperPressed,
}

pub struct GpioManager {
    wake_pin: u8,
    recovery_pin: u8,
}

impl GpioManager {
    pub fn new(wake_pin: u8, recovery_pin: u8) -> Self {
        Self {
            wake_pin,
            recovery_pin,
        }
    }

    pub fn is_recovery_jumper_active(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            if let Ok(gpio) = rppal::gpio::Gpio::new() {
                if let Ok(pin) = gpio.get(self.recovery_pin) {
                    let input = pin.into_input_pullup();
                    // Active low jumper (pulled to GND when recovery requested)
                    return input.is_low();
                }
            }
        }
        false
    }

    pub async fn run_wake_monitor(self, event_tx: mpsc::Sender<GpioEvent>) {
        info!(
            "Starting GPIO wake monitor on pin {} (Recovery check pin {})",
            self.wake_pin, self.recovery_pin
        );

        #[cfg(target_os = "linux")]
        {
            if let Ok(gpio) = rppal::gpio::Gpio::new() {
                if let Ok(pin) = gpio.get(self.wake_pin) {
                    let mut input = pin.into_input();
                    let _ = input.set_interrupt(rppal::gpio::Trigger::Both);

                    loop {
                        match input.poll_interrupt(true, None) {
                            Ok(Some(_)) => {
                                if input.is_high() {
                                    info!("GPIO rising edge detected: Presence active!");
                                    let _ = event_tx.send(GpioEvent::PresenceWakeTriggered).await;
                                } else {
                                    info!("GPIO falling edge detected: Presence cleared.");
                                    let _ = event_tx.send(GpioEvent::PresenceGone).await;
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                warn!("Error polling GPIO interrupt: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }
        }

        // Mock loop for non-Pi development / virtual environments
        info!("GPIO hardware interface operating in software emulation mode.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    }
}
