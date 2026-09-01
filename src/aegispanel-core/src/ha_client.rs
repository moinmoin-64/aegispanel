use crate::state_machine::AlarmStatus;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{error, info, warn};
use zeroize::Zeroize;

#[derive(Debug, Clone)]
pub enum HaEvent {
    Connected,
    Disconnected,
    AlarmStateUpdated(AlarmStatus),
    DisarmResult { success: bool, message: Option<String> },
}

pub struct DisarmRequest {
    pub pin: String,
    pub entity_id: String,
}

impl Drop for DisarmRequest {
    fn drop(&mut self) {
        self.pin.zeroize();
    }
}

pub struct HomeAssistantClient {
    url: String,
    token: String,
    alarmo_entity_id: String,
    event_tx: mpsc::Sender<HaEvent>,
    message_id: Arc<AtomicU64>,
}

impl HomeAssistantClient {
    pub fn new(
        url: String,
        token: String,
        alarmo_entity_id: String,
        event_tx: mpsc::Sender<HaEvent>,
    ) -> Self {
        Self {
            url,
            token,
            alarmo_entity_id,
            event_tx,
            message_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn run_loop(self: Arc<Self>, mut disarm_rx: mpsc::Receiver<DisarmRequest>) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(30);

        loop {
            let ws_url = format!("{}/api/websocket", self.url)
                .replace("https://", "wss://")
                .replace("http://", "ws://");

            info!("Connecting to Home Assistant WebSocket at {}", ws_url);

            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => {
                    info!("Connected to Home Assistant WebSocket. Performing handshake...");
                    backoff = Duration::from_secs(1);
                    let _ = self.event_tx.send(HaEvent::Connected).await;

                    let (mut write, mut read) = ws_stream.split();

                    // Step 1: Wait for auth_required from HA
                    let mut authenticated = false;
                    if let Some(Ok(Message::Text(txt))) = read.next().await {
                        if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                            if v["type"] == "auth_required" {
                                // Send auth message
                                let auth_msg = serde_json::json!({
                                    "type": "auth",
                                    "access_token": self.token
                                });
                                let _ = write.send(Message::Text(auth_msg.to_string())).await;

                                // Wait for auth_ok
                                if let Some(Ok(Message::Text(auth_res))) = read.next().await {
                                    if let Ok(res_val) = serde_json::from_str::<Value>(&auth_res) {
                                        if res_val["type"] == "auth_ok" {
                                            info!("Home Assistant authentication SUCCESS!");
                                            authenticated = true;
                                        } else {
                                            error!("Home Assistant authentication FAILED!");
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !authenticated {
                        let _ = self.event_tx.send(HaEvent::Disconnected).await;
                        tokio::time::sleep(backoff).await;
                        continue;
                    }

                    // Step 2: Subscribe to Alarmo entity state changes
                    let sub_id = self.message_id.fetch_add(1, Ordering::SeqCst);
                    let sub_msg = serde_json::json!({
                        "id": sub_id,
                        "type": "subscribe_events",
                        "event_type": "state_changed"
                    });
                    let _ = write.send(Message::Text(sub_msg.to_string())).await;

                    // Also query initial state of Alarmo entity
                    let state_req_id = self.message_id.fetch_add(1, Ordering::SeqCst);
                    let state_req = serde_json::json!({
                        "id": state_req_id,
                        "type": "get_states"
                    });
                    let _ = write.send(Message::Text(state_req.to_string())).await;

                    // Step 3: Event loop with Ping Heartbeat & Disarm handling
                    let mut ping_timer = interval(Duration::from_secs(10));
                    let mut pending_ping_id: Option<u64> = None;

                    loop {
                        tokio::select! {
                            _ = ping_timer.tick() => {
                                if pending_ping_id.is_some() {
                                    warn!("Ping timeout! HA WebSocket disconnected.");
                                    break;
                                }
                                let pid = self.message_id.fetch_add(1, Ordering::SeqCst);
                                let ping_msg = serde_json::json!({
                                    "id": pid,
                                    "type": "ping"
                                });
                                pending_ping_id = Some(pid);
                                if let Err(e) = write.send(Message::Text(ping_msg.to_string())).await {
                                    warn!("Failed to send ping: {}", e);
                                    break;
                                }
                            }

                            Some(disarm_req) = disarm_rx.recv() => {
                                let call_id = self.message_id.fetch_add(1, Ordering::SeqCst);
                                let service_data = if disarm_req.pin.is_empty() {
                                    serde_json::json!({})
                                } else {
                                    serde_json::json!({ "code": disarm_req.pin })
                                };
                                let call_msg = serde_json::json!({
                                    "id": call_id,
                                    "type": "call_service",
                                    "domain": "alarm_control_panel",
                                    "service": "alarm_disarm",
                                    "target": {
                                        "entity_id": if disarm_req.entity_id.is_empty() { self.alarmo_entity_id.clone() } else { disarm_req.entity_id.clone() }
                                    },
                                    "service_data": service_data
                                });
                                info!("Sending disarm service call to Alarmo (PIN zeroized after transmit)");
                                if let Err(e) = write.send(Message::Text(call_msg.to_string())).await {
                                    error!("Failed to send disarm command: {}", e);
                                    let _ = self.event_tx.send(HaEvent::DisarmResult {
                                        success: false,
                                        message: Some("Netzwerkfehler beim Senden".to_string())
                                    }).await;
                                }
                            }

                            msg_opt = read.next() => {
                                match msg_opt {
                                    Some(Ok(Message::Text(text))) => {
                                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                                            let msg_type = v["type"].as_str().unwrap_or("");
                                            
                                            if msg_type == "pong" {
                                                pending_ping_id = None;
                                            } else if msg_type == "event" {
                                                let entity_id = v["event"]["data"]["entity_id"].as_str().unwrap_or("");
                                                if entity_id == self.alarmo_entity_id {
                                                    let state_str = v["event"]["data"]["new_state"]["state"].as_str().unwrap_or("");
                                                    let alarm_status = parse_ha_state(state_str);
                                                    info!("Alarmo entity state changed: {}", state_str);
                                                    let _ = self.event_tx.send(HaEvent::AlarmStateUpdated(alarm_status)).await;
                                                }
                                            } else if msg_type == "result" {
                                                let result_id = v["id"].as_u64().unwrap_or(0);
                                                if result_id == state_req_id && v["success"].as_bool().unwrap_or(false) {
                                                    if let Some(arr) = v["result"].as_array() {
                                                        for item in arr {
                                                            if item["entity_id"].as_str().unwrap_or("") == self.alarmo_entity_id {
                                                                let st = item["state"].as_str().unwrap_or("");
                                                                let status = parse_ha_state(st);
                                                                info!("Initial Alarmo state fetched: {}", st);
                                                                let _ = self.event_tx.send(HaEvent::AlarmStateUpdated(status)).await;
                                                            }
                                                        }
                                                    }
                                                } else if v["success"].as_bool() == Some(false) {
                                                    let err_msg = v["error"]["message"].as_str().unwrap_or("PIN Falsch").to_string();
                                                    warn!("Alarmo service call failed: {}", err_msg);
                                                    let _ = self.event_tx.send(HaEvent::DisarmResult {
                                                        success: false,
                                                        message: Some(err_msg)
                                                    }).await;
                                                } else if v["success"].as_bool() == Some(true) {
                                                    let _ = self.event_tx.send(HaEvent::DisarmResult {
                                                        success: true,
                                                        message: None
                                                    }).await;
                                                }
                                            }
                                        }
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        warn!("WebSocket connection closed by server.");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        error!("WebSocket error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to HA WebSocket: {}", e);
                }
            }

            let _ = self.event_tx.send(HaEvent::Disconnected).await;
            info!("Retrying connection in {} seconds...", backoff.as_secs());
            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, max_backoff);
        }
    }
}

fn parse_ha_state(state: &str) -> AlarmStatus {
    match state {
        "disarmed" => AlarmStatus::Disarmed,
        "armed_home" => AlarmStatus::ArmedHome,
        "armed_away" => AlarmStatus::ArmedAway,
        "armed_night" => AlarmStatus::ArmedNight,
        "armed_vacation" => AlarmStatus::ArmedVacation,
        "arming" => AlarmStatus::Arming,
        "disarming" => AlarmStatus::Disarming,
        "pending" => AlarmStatus::Pending,
        "triggered" => AlarmStatus::Triggered,
        _ => AlarmStatus::Unknown,
    }
}
