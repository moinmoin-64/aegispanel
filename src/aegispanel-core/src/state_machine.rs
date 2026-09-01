#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemState {
    Boot,
    Initializing,
    Wizard,
    Connecting,
    CheckingAlarm,
    Security,      // Alarmo = armed_* / triggered
    Kiosk,         // Alarmo = disarmed
    Sleep,         // Display OFF (Night mode or inactivity)
    Waking,        // Presence detected, returning to active state
    Offline,       // Fail-safe mode (HA unreachable)
    Update,        // OTA update in progress
    Recovery,      // System recovery mode
    Error,         // Critical system error
}

impl std::fmt::Display for SystemState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemState::Boot => write!(f, "BOOT"),
            SystemState::Initializing => write!(f, "INITIALIZING"),
            SystemState::Wizard => write!(f, "WIZARD"),
            SystemState::Connecting => write!(f, "CONNECTING"),
            SystemState::CheckingAlarm => write!(f, "CHECKING_ALARM"),
            SystemState::Security => write!(f, "SECURITY"),
            SystemState::Kiosk => write!(f, "KIOSK"),
            SystemState::Sleep => write!(f, "SLEEP"),
            SystemState::Waking => write!(f, "WAKING"),
            SystemState::Offline => write!(f, "OFFLINE"),
            SystemState::Update => write!(f, "UPDATE"),
            SystemState::Recovery => write!(f, "RECOVERY"),
            SystemState::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StateEvent {
    InitComplete { configured: bool },
    WizardFinished,
    HaConnected,
    HaDisconnected,
    AlarmStateChanged(AlarmStatus),
    PresenceDetected,
    FaceRecognized { user: String },
    InactivityTimeout,
    NightModeStart,
    NightModeEnd,
    OtaStart,
    OtaComplete,
    OtaFailed,
    RecoveryTriggered,
    FatalError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmStatus {
    Disarmed,
    ArmedHome,
    ArmedAway,
    ArmedNight,
    ArmedVacation,
    Arming,
    Disarming,
    Pending,
    Triggered,
    Unknown,
}

impl AlarmStatus {
    pub fn is_armed(&self) -> bool {
        matches!(
            self,
            AlarmStatus::ArmedHome
                | AlarmStatus::ArmedAway
                | AlarmStatus::ArmedNight
                | AlarmStatus::ArmedVacation
                | AlarmStatus::Arming
                | AlarmStatus::Pending
                | AlarmStatus::Triggered
        )
    }
}

pub struct StateMachine {
    current_state: SystemState,
    last_active_state: SystemState,
    alarm_status: AlarmStatus,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: SystemState::Boot,
            last_active_state: SystemState::Security,
            alarm_status: AlarmStatus::Unknown,
        }
    }

    pub fn current_state(&self) -> SystemState {
        self.current_state
    }

    pub fn alarm_status(&self) -> &AlarmStatus {
        &self.alarm_status
    }

    pub fn transition(&mut self, event: StateEvent) -> SystemState {
        let old_state = self.current_state;

        let new_state = match (self.current_state, event.clone()) {
            (SystemState::Boot, StateEvent::InitComplete { configured: false }) => SystemState::Wizard,
            (SystemState::Boot, StateEvent::InitComplete { configured: true }) => SystemState::Connecting,
            
            (SystemState::Wizard, StateEvent::WizardFinished) => SystemState::Connecting,

            (SystemState::Connecting, StateEvent::HaConnected) => SystemState::CheckingAlarm,
            (SystemState::Connecting, StateEvent::HaDisconnected) => SystemState::Offline,
            (SystemState::Offline, StateEvent::HaConnected) => SystemState::CheckingAlarm,

            (SystemState::CheckingAlarm, StateEvent::AlarmStateChanged(status)) => {
                self.alarm_status = status.clone();
                if status.is_armed() {
                    SystemState::Security
                } else if status == AlarmStatus::Disarmed {
                    SystemState::Kiosk
                } else {
                    SystemState::Offline
                }
            }

            (SystemState::Kiosk | SystemState::Security, StateEvent::AlarmStateChanged(status)) => {
                self.alarm_status = status.clone();
                if status.is_armed() {
                    SystemState::Security
                } else if status == AlarmStatus::Disarmed {
                    SystemState::Kiosk
                } else {
                    SystemState::Offline
                }
            }

            // Face ID Recognition Auto-Disarm / Auto-Unlock
            (SystemState::Security | SystemState::Sleep | SystemState::Waking, StateEvent::FaceRecognized { ref user }) => {
                info!("Face ID matched for user '{}'! Auto-unlocking panel...", user);
                self.alarm_status = AlarmStatus::Disarmed;
                SystemState::Kiosk
            }

            (SystemState::Kiosk | SystemState::Security | SystemState::CheckingAlarm, StateEvent::HaDisconnected) => {
                SystemState::Offline
            }

            (SystemState::Kiosk | SystemState::Security, StateEvent::InactivityTimeout) => {
                self.last_active_state = self.current_state;
                SystemState::Sleep
            }
            (SystemState::Kiosk | SystemState::Security, StateEvent::NightModeStart) => {
                self.last_active_state = self.current_state;
                SystemState::Sleep
            }

            (SystemState::Sleep, StateEvent::AlarmStateChanged(status)) => {
                self.alarm_status = status.clone();
                if status.is_armed() {
                    info!("Alarm ARMED while in sleep! Waking up immediately to Security screen.");
                    SystemState::Security
                } else {
                    SystemState::Sleep
                }
            }

            (SystemState::Sleep, StateEvent::PresenceDetected) => SystemState::Waking,
            (SystemState::Sleep, StateEvent::NightModeEnd) => SystemState::Waking,

            (SystemState::Waking, _) => {
                if self.alarm_status.is_armed() {
                    SystemState::Security
                } else if self.alarm_status == AlarmStatus::Disarmed {
                    SystemState::Kiosk
                } else {
                    SystemState::Offline
                }
            }

            (_, StateEvent::OtaStart) => SystemState::Update,
            (SystemState::Update, StateEvent::OtaComplete) => SystemState::Connecting,
            (SystemState::Update, StateEvent::OtaFailed) => SystemState::Error,
            (_, StateEvent::RecoveryTriggered) => SystemState::Recovery,
            (_, StateEvent::FatalError(ref err)) => {
                warn!("Fatal error occurred: {}", err);
                SystemState::Error
            }

            (current, event) => {
                warn!("Unhandled state transition: {:?} with event {:?}", current, event);
                current
            }
        };

        if old_state != new_state {
            info!("State transition: {} -> {}", old_state, new_state);
            self.current_state = new_state;
        }

        self.current_state
    }
}
