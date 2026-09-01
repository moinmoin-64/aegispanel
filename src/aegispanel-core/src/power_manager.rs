#![allow(dead_code)]

use chrono::{Local, NaiveTime};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

pub struct PowerManager {
    night_mode_enabled: bool,
    night_start: NaiveTime,
    night_end: NaiveTime,
    inactivity_timeout_secs: u64,
    display_on: bool,
}

impl PowerManager {
    pub fn new(
        night_mode_enabled: bool,
        night_start_str: &str,
        night_end_str: &str,
        inactivity_timeout_secs: u64,
    ) -> Self {
        let night_start = NaiveTime::parse_from_str(night_start_str, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        let night_end = NaiveTime::parse_from_str(night_end_str, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(6, 30, 0).unwrap());

        Self {
            night_mode_enabled,
            night_start,
            night_end,
            inactivity_timeout_secs,
            display_on: true,
        }
    }

    pub fn set_display_power(&mut self, turn_on: bool) {
        if self.display_on == turn_on {
            return;
        }

        self.display_on = turn_on;
        let val = if turn_on { "0" } else { "1" }; // sysfs bl_power: 0 = ON, 1 = OFF

        // Try standard Linux sysfs backlight paths
        let sysfs_paths = [
            "/sys/class/backlight/rpi_backlight/bl_power",
            "/sys/class/backlight/10-0045/bl_power",
            "/sys/class/backlight/backlight/bl_power",
        ];

        let mut success = false;
        for path in sysfs_paths {
            if Path::new(path).exists() {
                if let Err(e) = fs::write(path, val) {
                    warn!("Failed to set display power at {}: {}", path, e);
                } else {
                    info!("Display backlight power set to {} ({})", turn_on, path);
                    success = true;
                    break;
                }
            }
        }

        if !success {
            // Fallback: try vcgencmd or xset if sysfs path is not directly mounted in dev environment
            info!("Sysfs backlight path not found. Mocking display power set to {}", turn_on);
        }
    }

    pub fn is_night_time(&self) -> bool {
        if !self.night_mode_enabled {
            return false;
        }

        let now = Local::now().time();
        if self.night_start > self.night_end {
            // Overnight span (e.g. 22:00 to 06:30)
            now >= self.night_start || now < self.night_end
        } else {
            now >= self.night_start && now < self.night_end
        }
    }

    pub fn is_display_on(&self) -> bool {
        self.display_on
    }

    pub fn inactivity_timeout_secs(&self) -> u64 {
        self.inactivity_timeout_secs
    }
}
