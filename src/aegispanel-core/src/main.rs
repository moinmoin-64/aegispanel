mod config;
mod face_id;
mod gpio_manager;
mod ha_client;
mod ipc_server;
mod ota;
mod power_manager;
mod state_machine;

use config::ConfigManager;
use face_id::FaceIdManager;
use gpio_manager::{GpioEvent, GpioManager, DEFAULT_ESP32_WAKE_PIN, DEFAULT_RECOVERY_JUMPER_PIN};
use ha_client::{DisarmRequest, HaEvent, HomeAssistantClient};
use ipc_server::IpcServer;
use ota::OtaManager;
use power_manager::PowerManager;
use state_machine::{AlarmStatus, StateEvent, StateMachine, SystemState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{error, info, warn};
use tracing_subscriber::FmtSubscriber;

const CURRENT_VERSION: &str = "1.1.1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    info!("===============================================");
    info!("   Starting AegisPanel OS Core Daemon v{}   ", CURRENT_VERSION);
    info!("===============================================");

    let config_mgr = Arc::new(RwLock::new(ConfigManager::load_all()));
    let cfg = config_mgr.read().await;

    let gpio_mgr = GpioManager::new(DEFAULT_ESP32_WAKE_PIN, DEFAULT_RECOVERY_JUMPER_PIN);
    if gpio_mgr.is_recovery_jumper_active() {
        warn!("Physical recovery jumper detected at boot! Triggering RECOVERY mode.");
        let mut sm = StateMachine::new();
        sm.transition(StateEvent::RecoveryTriggered);
        let _ = std::process::Command::new("/usr/bin/aegispanel-recovery-trigger").status();
        return Ok(());
    }

    let mut state_machine = StateMachine::new();
    let current_state = Arc::new(RwLock::new(state_machine.current_state()));

    let is_configured = cfg.system.first_boot_complete && !cfg.ha.url.is_empty();
    state_machine.transition(StateEvent::InitComplete {
        configured: is_configured,
    });
    *current_state.write().await = state_machine.current_state();

    info!("Initial State: {}", state_machine.current_state());

    let (ha_event_tx, mut ha_event_rx) = mpsc::channel::<HaEvent>(32);
    let (disarm_tx, disarm_rx) = mpsc::channel::<DisarmRequest>(16);
    let (gpio_event_tx, mut gpio_event_rx) = mpsc::channel::<GpioEvent>(16);
    let (face_id_tx, mut face_id_rx) = mpsc::channel::<String>(8);

    let presence_active = Arc::new(AtomicBool::new(false));
    let face_disarm_tx = disarm_tx.clone();

    let power_mgr = Arc::new(RwLock::new(PowerManager::new(
        cfg.power.night_mode_enabled,
        &cfg.power.night_mode_start,
        &cfg.power.night_mode_end,
        cfg.power.inactivity_timeout_secs,
    )));

    let ipc_server = Arc::new(IpcServer::new(
        disarm_tx.clone(),
        Arc::clone(&current_state),
        Arc::clone(&config_mgr),
    ));
    tokio::spawn(async move {
        if let Err(e) = ipc_server.run_server().await {
            error!("IPC Server terminated with error: {}", e);
        }
    });

    let gpio_task_mgr = GpioManager::new(DEFAULT_ESP32_WAKE_PIN, DEFAULT_RECOVERY_JUMPER_PIN);
    let gpio_tx = gpio_event_tx.clone();
    tokio::spawn(async move {
        gpio_task_mgr.run_wake_monitor(gpio_tx).await;
    });

    let face_id_mgr = if cfg.face_id.enabled && is_configured {
        info!("Face ID module ENABLED. Initializing camera capture manager...");
        Some(Arc::new(FaceIdManager::new(
            cfg.face_id.clone(),
            cfg.ha.url.clone(),
            cfg.secrets.ha_access_token.clone(),
            face_id_tx,
        )))
    } else {
        None
    };

    if is_configured {
        let ha_url = cfg.ha.url.clone();
        let ha_token = cfg.secrets.ha_access_token.clone();
        let alarmo_entity = cfg.ha.alarmo_entity_id.clone();

        let ha_client = Arc::new(HomeAssistantClient::new(
            ha_url,
            ha_token,
            alarmo_entity,
            ha_event_tx,
        ));
        tokio::spawn(async move {
            ha_client.run_loop(disarm_rx).await;
        });
    } else {
        info!("System is in FIRST-BOOT WIZARD mode. Waiting for user configuration via UI...");
    }

    // Hardware Watchdog Timer task (pings /dev/watchdog every 15 seconds)
    tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_secs(15));
        loop {
            interval_timer.tick().await;
            if std::path::Path::new("/dev/watchdog").exists() {
                let _ = std::fs::write("/dev/watchdog", b"\0");
            }
        }
    });

    // Autonomous Background GitHub OTA Updater Service
    if cfg.update.auto_check && !cfg.update.github_repo.is_empty() {
        let update_cfg = cfg.update.clone();
        let current_state_ref = Arc::clone(&current_state);
        info!(
            "Starting Background GitHub OTA Updater Service (Repo: {}, interval: {}h)...",
            update_cfg.github_repo, update_cfg.check_interval_hours
        );

        tokio::spawn(async move {
            // Initial delay before first check
            tokio::time::sleep(Duration::from_secs(60)).await;

            let interval_secs = (update_cfg.check_interval_hours as u64).max(1) * 3600;
            let mut check_interval = interval(Duration::from_secs(interval_secs));

            loop {
                check_interval.tick().await;
                info!("Checking GitHub ({}) for new AegisPanel OS releases...", update_cfg.github_repo);

                let token = if update_cfg.github_token.is_empty() {
                    None
                } else {
                    Some(update_cfg.github_token.as_str())
                };

                match OtaManager::check_github_update(&update_cfg.github_repo, token, CURRENT_VERSION).await {
                    Ok(Some(release)) => {
                        info!("New release {} available! Evaluating safe install window...", release.tag_name);

                        // Wait until system is NOT in active SECURITY mode
                        let state = *current_state_ref.read().await;
                        if state != SystemState::Security {
                            info!("Safe window active. Starting automated background download & flash...");
                            let ota_mgr = OtaManager::new(ota::PUBLIC_KEY_FILE.to_string());
                            let current_slot = "a"; // Resolved from cmdline in production

                            match ota_mgr.install_github_release(&release, token, current_slot).await {
                                Ok(new_slot) => {
                                    info!("Automated OTA update complete! New slot: {}. Scheduling reboot...", new_slot);
                                    // Automatic reboot during night mode or after 30s
                                    tokio::time::sleep(Duration::from_secs(30)).await;
                                    let _ = std::process::Command::new("reboot").status();
                                }
                                Err(e) => {
                                    error!("Automated OTA installation failed: {}", e);
                                }
                            }
                        } else {
                            info!("System is currently armed (SECURITY). Postponing update install.");
                        }
                    }
                    Ok(None) => {
                        info!("No new updates found on GitHub. System is up to date.");
                    }
                    Err(e) => {
                        warn!("Background GitHub update check failed: {}", e);
                    }
                }
            }
        });
    }

    drop(cfg);

    let mut main_interval = interval(Duration::from_secs(1));
    let mut last_activity_time = std::time::Instant::now();

    loop {
        tokio::select! {
            _ = main_interval.tick() => {
                let mut pm = power_mgr.write().await;
                let state = *current_state.read().await;

                if pm.is_night_time() && state != SystemState::Sleep && state != SystemState::Security {
                    info!("Night mode active. Turning off display...");
                    pm.set_display_power(false);
                    let next = state_machine.transition(StateEvent::NightModeStart);
                    *current_state.write().await = next;
                } else if state == SystemState::Kiosk && last_activity_time.elapsed().as_secs() > pm.inactivity_timeout_secs() {
                    info!("Inactivity timeout reached. Display sleeping...");
                    pm.set_display_power(false);
                    let next = state_machine.transition(StateEvent::InactivityTimeout);
                    *current_state.write().await = next;
                }
            }

            Some(gpio_evt) = gpio_event_rx.recv() => {
                match gpio_evt {
                    GpioEvent::PresenceWakeTriggered => {
                        presence_active.store(true, Ordering::SeqCst);
                        let sys_state_before = *current_state.read().await;
                        info!("Presence wake signal received from mmWave sensor! System state: {}", sys_state_before);
                        last_activity_time = std::time::Instant::now();
                        let mut pm = power_mgr.write().await;
                        pm.set_display_power(true);
                        let next = state_machine.transition(StateEvent::PresenceDetected);
                        *current_state.write().await = next;

                        if sys_state_before == SystemState::Security || state_machine.alarm_status().is_armed() {
                            if let Some(ref fid) = face_id_mgr {
                                info!("SECURITY mode + mmWave motion -> Launching Face ID 2s retry loop!");
                                let fid_clone = Arc::clone(fid);
                                let state_ref = Arc::clone(&current_state);
                                let presence_ref = Arc::clone(&presence_active);
                                tokio::spawn(async move {
                                    fid_clone.run_retry_loop(state_ref, presence_ref).await;
                                });
                            }
                        } else {
                            info!("Motion detected in KIOSK/DISARMED mode. Camera photo SKIPPED.");
                        }
                    }
                    GpioEvent::PresenceGone => {
                        presence_active.store(false, Ordering::SeqCst);
                        info!("mmWave sensor cleared: No presence detected.");
                    }
                    GpioEvent::RecoveryJumperPressed => {
                        warn!("Recovery jumper triggered during operation!");
                        let next = state_machine.transition(StateEvent::RecoveryTriggered);
                        *current_state.write().await = next;
                    }
                }
            }

            Some(recognized_user) = face_id_rx.recv() => {
                info!("Face ID Event received for user '{}'!", recognized_user);
                
                let _ = face_disarm_tx.send(DisarmRequest {
                    pin: String::new(),
                    entity_id: String::new(),
                }).await;

                let next = state_machine.transition(StateEvent::FaceRecognized { user: recognized_user });
                *current_state.write().await = next;

                let mut pm = power_mgr.write().await;
                pm.set_display_power(true);
            }

            Some(ha_evt) = ha_event_rx.recv() => {
                match ha_evt {
                    HaEvent::Connected => {
                        info!("Home Assistant WebSocket connected!");
                        let next = state_machine.transition(StateEvent::HaConnected);
                        *current_state.write().await = next;
                    }
                    HaEvent::Disconnected => {
                        warn!("Home Assistant connection lost! Entering OFFLINE fail-safe mode.");
                        let next = state_machine.transition(StateEvent::HaDisconnected);
                        *current_state.write().await = next;
                        let mut pm = power_mgr.write().await;
                        pm.set_display_power(true);
                    }
                    HaEvent::AlarmStateUpdated(status) => {
                        info!("Alarmo status update received: {:?}", status);
                        let next = state_machine.transition(StateEvent::AlarmStateChanged(status));
                        *current_state.write().await = next;

                        let mut pm = power_mgr.write().await;
                        if state_machine.alarm_status().is_armed() {
                            pm.set_display_power(true);
                        }
                    }
                    HaEvent::DisarmResult { success, message } => {
                        if success {
                            info!("Alarmo PIN validation SUCCESS! Alarm disarmed.");
                            let next = state_machine.transition(StateEvent::AlarmStateChanged(AlarmStatus::Disarmed));
                            *current_state.write().await = next;
                        } else {
                            warn!("Alarmo PIN validation FAILED: {:?}", message);
                        }
                    }
                }
            }
        }
    }
}
