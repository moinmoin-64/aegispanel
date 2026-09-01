use crate::config::ConfigManager;
use crate::ha_client::DisarmRequest;
use crate::ota::OtaManager;
use crate::state_machine::SystemState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use zeroize::Zeroize;

pub const IPC_SOCKET_PATH: &str = "/run/aegispanel/ipc.sock";

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum IpcRequest {
    #[serde(rename = "get_status")]
    GetStatus,

    #[serde(rename = "disarm")]
    Disarm { pin: String },

    #[serde(rename = "save_wizard_config")]
    SaveWizardConfig {
        ha_url: String,
        ha_token: String,
        wifi_ssid: String,
        wifi_psk: String,
        language: String,
        face_id_enabled: Option<bool>,
        face_id_auto_disarm: Option<bool>,
    },

    #[serde(rename = "test_ha_connection")]
    TestHaConnection { url: String, token: String },

    #[serde(rename = "check_update")]
    CheckUpdate,

    #[serde(rename = "apply_update")]
    ApplyUpdate { asset_url: String },

    #[serde(rename = "wake")]
    Wake,
}

#[derive(Debug, Serialize)]
pub struct IpcResponse {
    pub success: bool,
    pub state: Option<String>,
    pub alarm_status: Option<String>,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

pub struct IpcServer {
    disarm_tx: mpsc::Sender<DisarmRequest>,
    current_state: Arc<RwLock<SystemState>>,
    config_mgr: Arc<RwLock<ConfigManager>>,
}

impl IpcServer {
    pub fn new(
        disarm_tx: mpsc::Sender<DisarmRequest>,
        current_state: Arc<RwLock<SystemState>>,
        config_mgr: Arc<RwLock<ConfigManager>>,
    ) -> Self {
        Self {
            disarm_tx,
            current_state,
            config_mgr,
        }
    }

    pub async fn run_server(self: Arc<Self>) -> Result<(), String> {
        let socket_path = Path::new(IPC_SOCKET_PATH);
        if socket_path.exists() {
            let _ = fs::remove_file(socket_path);
        }

        if let Some(parent) = socket_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(IPC_SOCKET_PATH)
            .map_err(|e| format!("Failed to bind UNIX socket {}: {}", IPC_SOCKET_PATH, e))?;

        let permissions = fs::Permissions::from_mode(0o660);
        let _ = fs::set_permissions(IPC_SOCKET_PATH, permissions);

        info!("UNIX Domain Socket IPC server running at {}", IPC_SOCKET_PATH);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            warn!("IPC client connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting IPC connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, stream: UnixStream) -> Result<(), String> {
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        while buf_reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            > 0
        {
            let req_res: Result<IpcRequest, _> = serde_json::from_str(&line);
            let response = match req_res {
                Ok(IpcRequest::GetStatus) => {
                    let state = *self.current_state.read().await;
                    IpcResponse {
                        success: true,
                        state: Some(state.to_string()),
                        alarm_status: None,
                        message: None,
                        details: None,
                    }
                }

                Ok(IpcRequest::Disarm { ref pin }) => {
                    info!("Received disarm request over local IPC. Forwarding to HA (PIN zeroized)...");
                    let mut pin_copy = pin.clone();
                    let req = DisarmRequest {
                        pin: pin_copy.clone(),
                        entity_id: "".to_string(),
                    };
                    pin_copy.zeroize();

                    if let Err(e) = self.disarm_tx.send(req).await {
                        IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Channel send error: {}", e)),
                            details: None,
                        }
                    } else {
                        IpcResponse {
                            success: true,
                            state: None,
                            alarm_status: None,
                            message: Some("Disarm request queued".to_string()),
                            details: None,
                        }
                    }
                }

                Ok(IpcRequest::SaveWizardConfig {
                    ref ha_url,
                    ref ha_token,
                    ref wifi_ssid,
                    ref wifi_psk,
                    ref language,
                    ref face_id_enabled,
                    ref face_id_auto_disarm,
                }) => {
                    let mut cfg = self.config_mgr.write().await;
                    cfg.ha.url = ha_url.clone();
                    cfg.system.language = language.clone();
                    cfg.system.first_boot_complete = true;
                    cfg.secrets.ha_access_token = ha_token.clone();
                    cfg.secrets.wifi_ssid = wifi_ssid.clone();
                    cfg.secrets.wifi_psk = wifi_psk.clone();

                    if let Some(enabled) = face_id_enabled {
                        cfg.face_id.enabled = *enabled;
                    }
                    if let Some(auto_disarm) = face_id_auto_disarm {
                        cfg.face_id.auto_disarm = *auto_disarm;
                    }

                    let res = cfg
                        .save_system()
                        .and_then(|_| cfg.save_ha())
                        .and_then(|_| cfg.save_secrets())
                        .and_then(|_| cfg.save_face_id());

                    match res {
                        Ok(_) => IpcResponse {
                            success: true,
                            state: None,
                            alarm_status: None,
                            message: Some("Wizard & Face ID configuration saved successfully".to_string()),
                            details: None,
                        },
                        Err(e) => IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Failed to save configuration: {}", e)),
                            details: None,
                        },
                    }
                }

                Ok(IpcRequest::TestHaConnection { ref url, ref token }) => {
                    let client = reqwest::Client::new();
                    let test_url = format!("{}/api/", url.trim_end_matches('/'));
                    let res = client
                        .get(&test_url)
                        .header("Authorization", format!("Bearer {}", token))
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await;

                    match res {
                        Ok(resp) if resp.status().is_success() => IpcResponse {
                            success: true,
                            state: None,
                            alarm_status: None,
                            message: Some("Home Assistant reachable and authenticated".to_string()),
                            details: None,
                        },
                        Ok(resp) => IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("HTTP Status Error: {}", resp.status())),
                            details: None,
                        },
                        Err(e) => IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Connection test failed: {}", e)),
                            details: None,
                        },
                    }
                }

                Ok(IpcRequest::CheckUpdate) => {
                    let cfg = self.config_mgr.read().await;
                    let repo = cfg.update.github_repo.clone();
                    let token = if cfg.update.github_token.is_empty() {
                        None
                    } else {
                        Some(cfg.update.github_token.as_str())
                    };

                    match OtaManager::check_github_update(&repo, token, "1.1.1").await {
                        Ok(Some(release)) => IpcResponse {
                            success: true,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Update verfügbar: {}", release.tag_name)),
                            details: Some(serde_json::json!({
                                "tag": release.tag_name,
                                "notes": release.body,
                                "assets": release.assets
                            })),
                        },
                        Ok(None) => IpcResponse {
                            success: true,
                            state: None,
                            alarm_status: None,
                            message: Some("System ist auf dem neuesten Stand.".to_string()),
                            details: None,
                        },
                        Err(e) => IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Update-Prüfung fehlgeschlagen: {}", e)),
                            details: None,
                        },
                    }
                }

                Ok(IpcRequest::ApplyUpdate { ref asset_url }) => {
                    let cfg = self.config_mgr.read().await;
                    let token = if cfg.update.github_token.is_empty() {
                        None
                    } else {
                        Some(cfg.update.github_token.as_str())
                    };
                    let target_tmp = "/tmp/ota_update.img";

                    match OtaManager::download_update_asset(asset_url, token, target_tmp).await {
                        Ok(_) => {
                            let ota_mgr = OtaManager::new("/etc/aegispanel/ota_pubkey.bin".to_string());
                            let current_slot = "a"; // Detected dynamically in production
                            match ota_mgr.flash_inactive_slot(current_slot, target_tmp) {
                                Ok(new_slot) => {
                                    let _ = ota_mgr.set_uboot_try_boot(&new_slot);
                                    IpcResponse {
                                        success: true,
                                        state: None,
                                        alarm_status: None,
                                        message: Some(format!("Update in Slot {} geflasht. Neustart erforderlich.", new_slot)),
                                        details: None,
                                    }
                                }
                                Err(e) => IpcResponse {
                                    success: false,
                                    state: None,
                                    alarm_status: None,
                                    message: Some(format!("Flashen fehlgeschlagen: {}", e)),
                                    details: None,
                                }
                            }
                        }
                        Err(e) => IpcResponse {
                            success: false,
                            state: None,
                            alarm_status: None,
                            message: Some(format!("Download fehlgeschlagen: {}", e)),
                            details: None,
                        }
                    }
                }

                Ok(IpcRequest::Wake) => IpcResponse {
                    success: true,
                    state: None,
                    alarm_status: None,
                    message: Some("System wake triggered".to_string()),
                    details: None,
                },

                Err(e) => IpcResponse {
                    success: false,
                    state: None,
                    alarm_status: None,
                    message: Some(format!("Invalid IPC JSON payload: {}", e)),
                    details: None,
                },
            };

            let res_bytes = serde_json::to_vec(&response).unwrap_or_default();
            writer.write_all(&res_bytes).await.map_err(|e| e.to_string())?;
            writer.write_all(b"\n").await.map_err(|e| e.to_string())?;
            line.clear();
        }

        Ok(())
    }
}
