#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tracing::{info, warn};

pub const CONFIG_DIR: &str = "/etc/aegispanel";
pub const SECRETS_FILE: &str = "/etc/aegispanel/secrets.json";
pub const SYSTEM_CONFIG_FILE: &str = "/etc/aegispanel/system.json";
pub const POWER_CONFIG_FILE: &str = "/etc/aegispanel/power.json";
pub const HA_CONFIG_FILE: &str = "/etc/aegispanel/homeassistant.json";
pub const UPDATE_CONFIG_FILE: &str = "/etc/aegispanel/update.json";
pub const FACE_ID_CONFIG_FILE: &str = "/etc/aegispanel/faceid.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub language: String,
    pub hostname: String,
    pub first_boot_complete: bool,
    pub kiosk_url: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            language: "de".to_string(),
            hostname: "aegispanel".to_string(),
            first_boot_complete: false,
            kiosk_url: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    pub night_mode_enabled: bool,
    pub night_mode_start: String, // e.g. "22:00"
    pub night_mode_end: String,   // e.g. "06:30"
    pub inactivity_timeout_secs: u64, // e.g. 60
    pub mmwave_wake_enabled: bool,
    pub mmwave_distance_cm: u16,   // e.g. 100
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            night_mode_enabled: true,
            night_mode_start: "22:00".to_string(),
            night_mode_end: "06:30".to_string(),
            inactivity_timeout_secs: 60,
            mmwave_wake_enabled: true,
            mmwave_distance_cm: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantConfig {
    pub url: String, // e.g. "https://ha.example.com" or "http://192.168.1.50:8123"
    pub alarmo_entity_id: String, // default: "alarm_control_panel.alarmo"
}

impl Default for HomeAssistantConfig {
    fn default() -> Self {
        Self {
            url: "".to_string(),
            alarmo_entity_id: "alarm_control_panel.alarmo".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceIdConfig {
    pub enabled: bool,
    pub camera_device: String, // e.g. "/dev/video0"
    pub auto_disarm: bool,
    pub confidence_threshold: f32, // e.g. 0.80
}

impl Default for FaceIdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            camera_device: "/dev/video0".to_string(),
            auto_disarm: true,
            confidence_threshold: 0.80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub github_repo: String, // e.g. "owner/panel"
    pub github_token: String, // Optional PAT for private GitHub repo
    pub channel: String, // "stable", "beta"
    pub auto_check: bool,
    pub check_interval_hours: u32,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            github_repo: "moinmoin-64/aegispanel".to_string(),
            github_token: "".to_string(),
            channel: "stable".to_string(),
            auto_check: true,
            check_interval_hours: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    pub ha_access_token: String,
    pub wifi_ssid: String,
    pub wifi_psk: String,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            ha_access_token: "".to_string(),
            wifi_ssid: "".to_string(),
            wifi_psk: "".to_string(),
        }
    }
}

pub struct ConfigManager {
    pub system: SystemConfig,
    pub power: PowerConfig,
    pub ha: HomeAssistantConfig,
    pub face_id: FaceIdConfig,
    pub update: UpdateConfig,
    pub secrets: SecretsConfig,
}

impl ConfigManager {
    pub fn load_all() -> Self {
        let _ = fs::create_dir_all(CONFIG_DIR);

        let system = Self::load_json(SYSTEM_CONFIG_FILE).unwrap_or_default();
        let power = Self::load_json(POWER_CONFIG_FILE).unwrap_or_default();
        let ha = Self::load_json(HA_CONFIG_FILE).unwrap_or_default();
        let face_id = Self::load_json(FACE_ID_CONFIG_FILE).unwrap_or_default();
        let update = Self::load_json(UPDATE_CONFIG_FILE).unwrap_or_default();
        let secrets = Self::load_secrets().unwrap_or_default();

        Self {
            system,
            power,
            ha,
            face_id,
            update,
            secrets,
        }
    }

    fn load_json<T: for<'a> Deserialize<'a>>(path: &str) -> Option<T> {
        if Path::new(path).exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(val) => Some(val),
                    Err(e) => {
                        warn!("Failed to parse config file {}: {}", path, e);
                        None
                    }
                },
                Err(e) => {
                    warn!("Failed to read config file {}: {}", path, e);
                    None
                }
            }
        } else {
            None
        }
    }

    fn load_secrets() -> Option<SecretsConfig> {
        Self::load_json(SECRETS_FILE)
    }

    pub fn save_secrets(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.secrets)
            .map_err(|e| format!("JSON serialize error: {}", e))?;

        fs::write(SECRETS_FILE, content).map_err(|e| format!("Write secrets error: {}", e))?;

        let permissions = Permissions::from_mode(0o600);
        let _ = fs::set_permissions(SECRETS_FILE, permissions);

        info!("Secrets configuration saved securely with 0600 permissions.");
        Ok(())
    }

    pub fn save_system(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.system)
            .map_err(|e| format!("JSON serialize error: {}", e))?;
        fs::write(SYSTEM_CONFIG_FILE, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    pub fn save_ha(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.ha)
            .map_err(|e| format!("JSON serialize error: {}", e))?;
        fs::write(HA_CONFIG_FILE, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    pub fn save_face_id(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.face_id)
            .map_err(|e| format!("JSON serialize error: {}", e))?;
        fs::write(FACE_ID_CONFIG_FILE, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    pub fn save_update(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.update)
            .map_err(|e| format!("JSON serialize error: {}", e))?;
        fs::write(UPDATE_CONFIG_FILE, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }
}
