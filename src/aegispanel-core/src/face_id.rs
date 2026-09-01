use crate::config::FaceIdConfig;
use crate::state_machine::SystemState;
use reqwest::multipart;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

pub struct FaceIdManager {
    config: FaceIdConfig,
    ha_url: String,
    ha_token: String,
    event_tx: mpsc::Sender<String>,
}

#[derive(Debug, Deserialize)]
struct DoubleTakeMatch {
    name: String,
    match_score: Option<f32>,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct DoubleTakeResponse {
    match_user: Option<String>,
    matches: Option<Vec<DoubleTakeMatch>>,
}

impl FaceIdManager {
    pub fn new(
        config: FaceIdConfig,
        ha_url: String,
        ha_token: String,
        event_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            config,
            ha_url,
            ha_token,
            event_tx,
        }
    }

    pub async fn run_retry_loop(
        &self,
        current_state: Arc<RwLock<SystemState>>,
        presence_active: Arc<AtomicBool>,
    ) {
        if !self.config.enabled {
            return;
        }

        info!("Starting Face ID retry loop (2s interval while in SECURITY mode & motion detected)...");

        loop {
            // Check condition 1: Must be in SECURITY mode
            let sys_state = *current_state.read().await;
            if sys_state != SystemState::Security {
                info!("System state is no longer SECURITY (state={}). Stopping Face ID loop.", sys_state);
                break;
            }

            // Check condition 2: Motion/presence must still be active
            if !presence_active.load(Ordering::SeqCst) {
                info!("No motion detected anymore. Stopping Face ID loop.");
                break;
            }

            // Capture and attempt recognition
            self.trigger_capture_and_recognize().await;

            // Wait 2 seconds before next retry check
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    pub async fn trigger_capture_and_recognize(&self) {
        if !self.config.enabled {
            return;
        }

        let dev_path = &self.config.camera_device;
        if !Path::new(dev_path).exists() {
            warn!("Face ID camera device {} not found. Skipping capture.", dev_path);
            return;
        }

        info!("Capturing frame from CSI camera ({}) for Face ID recognition...", dev_path);
        let snapshot_path = "/tmp/face_snapshot.jpg";

        let status = Command::new("v4l2-ctl")
            .args(&["--device", dev_path, "--set-fmt-video=width=640,height=480,pixelformat=MJPG", "--stream-mmap", "--stream-count=1", "--stream-to", snapshot_path])
            .status();

        let capture_ok = match status {
            Ok(s) if s.success() => true,
            _ => {
                Command::new("ffmpeg")
                    .args(&["-y", "-f", "v4l2", "-video_size", "640x480", "-i", dev_path, "-frames:v", "1", snapshot_path])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
        };

        if !capture_ok || !Path::new(snapshot_path).exists() {
            warn!("Failed to capture image snapshot from {}", dev_path);
            return;
        }

        info!("Snapshot captured. Uploading to Home Assistant AI Face ID service...");

        let image_bytes = match tokio::fs::read(snapshot_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read snapshot file: {}", e);
                return;
            }
        };

        let client = reqwest::Client::new();
        let api_url = format!("{}/api/app/double_take/api/recognize", self.ha_url.trim_end_matches('/'));

        let part = multipart::Part::bytes(image_bytes)
            .file_name("snapshot.jpg")
            .mime_str("image/jpeg")
            .unwrap_or_else(|_| multipart::Part::bytes(vec![]));

        let form = multipart::Form::new().part("file", part);

        let res = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", self.ha_token))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await;

        match res {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<DoubleTakeResponse>().await {
                    if let Some(user) = data.match_user {
                        info!("Face ID MATCH SUCCESS! Identified user: '{}'", user);
                        let _ = self.event_tx.send(user).await;
                    } else if let Some(matches) = data.matches {
                        if let Some(m) = matches.first() {
                            let conf = m.confidence.or(m.match_score).unwrap_or(0.0);
                            if conf >= self.config.confidence_threshold {
                                info!("Face ID MATCH SUCCESS! Identified user: '{}' (confidence: {:.2})", m.name, conf);
                                let _ = self.event_tx.send(m.name.clone()).await;
                            } else {
                                info!("Face detected ('{}') but confidence {:.2} below threshold {:.2}", m.name, conf, self.config.confidence_threshold);
                            }
                        }
                    } else {
                        info!("Face ID scanned – No matching user recognized.");
                    }
                }
            }
            Ok(resp) => {
                warn!("Face ID HTTP API returned status: {}", resp.status());
            }
            Err(e) => {
                warn!("Face ID HTTP request failed: {}", e);
            }
        }
    }
}
