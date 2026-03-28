use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::{ConnectionType, Waypoint};

/// Commands that can be sent from GCS to the vehicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GcsCommand {
    /// Arm or disarm motors
    Arm(bool),
    /// Set flight mode
    SetMode(u32),
    /// Upload a mission (list of waypoints)
    UploadMission(Vec<Waypoint>),
    /// Start the uploaded mission
    StartMission,
    /// Pause the current mission
    PauseMission,
    /// Resume a paused mission
    ResumeMission,
    /// Return to launch point
    ReturnToLaunch,
    /// Set guided mode target position
    GuidedGoto {
        lat: f64,
        lon: f64,
        alt: f32,
    },
    /// Override RC channel values (for joystick control)
    RcOverride {
        channels: [u16; 8],
    },
    /// Request data stream at a given rate (Hz)
    RequestDataStream {
        stream_id: u8,
        rate_hz: u16,
    },
    /// Set a specific ArduPilot parameter
    SetParam {
        param_id: String,
        value: f32,
    },
    /// Request to read a parameter value
    GetParam {
        param_id: String,
    },
    /// Trigger camera shutter
    CameraTrigger,
    /// Reboot the flight controller
    Reboot,
}

/// Events received from the vehicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VehicleEvent {
    /// Connection established
    Connected {
        connection: ConnectionType,
        system_id: u8,
        component_id: u8,
    },
    /// Connection lost
    Disconnected {
        reason: String,
    },
    /// Heartbeat timeout warning
    HeartbeatTimeout {
        last_seen_ms: u64,
    },
    /// Status text message from FC
    StatusText {
        severity: u8,
        text: String,
    },
    /// Mission acknowledged
    MissionAck {
        result: u8,
    },
    /// Parameter value received
    ParamValue {
        param_id: String,
        value: f32,
        param_type: u8,
        param_count: u16,
        param_index: u16,
    },
    /// Failsafe triggered
    Failsafe {
        reason: FailsafeReason,
    },
    /// Armed/Disarmed state changed
    ArmStateChanged {
        armed: bool,
    },
    /// Flight mode changed
    ModeChanged {
        mode: u32,
    },
}

/// Reasons for failsafe activation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FailsafeReason {
    RcLost,
    GcsLost,
    BatteryLow,
    BatteryCritical,
    GpsLost,
    GeofenceBreach,
}

/// Protocol errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Heartbeat timeout after {0}ms")]
    HeartbeatTimeout(u64),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Serial port error: {0}")]
    SerialError(String),

    #[error("Command rejected: {0}")]
    CommandRejected(String),

    #[error("Mission upload failed: {0}")]
    MissionUploadFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Commands sent from the UI thread to the background MAVLink thread
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// Connect to a flight controller
    Connect(crate::types::ConnectionType),
    /// Disconnect from the flight controller
    Disconnect,
    /// Send a MAVLink command to the vehicle
    SendCommand(GcsCommand),
}

/// Events sent from the background MAVLink thread back to the UI
#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// Successfully connected
    Connected,
    /// Disconnected (with reason)
    Disconnected(String),
    /// Error occurred
    Error(String),
    /// Telemetry data updated (clone of latest snapshot)
    TelemetryUpdated,
    /// Vehicle event (heartbeat, status text, etc.)
    Vehicle(VehicleEvent),
}
