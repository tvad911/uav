use serde::{Deserialize, Serialize};

/// Status of the companion computer (Samsung S20)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanionStatus {
    /// Whether the companion computer is connected
    pub connected: bool,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// CPU temperature in Celsius
    pub cpu_temp: f32,
    /// RAM usage in MB
    pub ram_used_mb: u32,
    /// RAM total in MB
    pub ram_total_mb: u32,
    /// 4G signal strength in dBm (e.g. -70 = good, -100 = poor)
    pub signal_dbm: i32,
    /// 4G network type (LTE / 5G / 3G)
    pub network_type: String,
    /// Data upload rate in KB/s
    pub upload_kbps: f32,
    /// Data download rate in KB/s
    pub download_kbps: f32,
    /// Battery percentage of the S20 (0-100, or -1 if powered externally)
    pub battery_pct: i8,
    /// Whether AI detection is actively running
    pub ai_running: bool,
    /// AI inference FPS
    pub ai_fps: f32,
    /// Whether video streaming is active
    pub streaming: bool,
    /// Companion GPS position (independent from FC GPS)
    pub gps_lat: f64,
    pub gps_lon: f64,
    /// Uptime in seconds
    pub uptime_s: u64,
    /// Last heartbeat timestamp (Unix epoch seconds)
    pub last_heartbeat: u64,
}

/// A single detection result from AI object detection on the companion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Class label (e.g., "person", "car", "truck")
    pub label: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Bounding box in normalized coordinates (0.0 to 1.0)
    pub bbox: BoundingBox,
    /// Distance estimate in meters (from monocular depth, if available)
    pub distance_m: Option<f32>,
}

/// Normalized bounding box (values 0.0 - 1.0 relative to frame size)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Top-left X
    pub x: f32,
    /// Top-left Y
    pub y: f32,
    /// Width
    pub w: f32,
    /// Height
    pub h: f32,
}

/// A frame of detection results from the companion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionFrame {
    /// Timestamp of the detection (Unix epoch millis)
    pub timestamp_ms: u64,
    /// List of detected objects
    pub detections: Vec<DetectionResult>,
    /// Source frame dimensions
    pub frame_width: u32,
    pub frame_height: u32,
}

/// Messages sent from companion to GCS via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompanionMessage {
    /// Periodic status heartbeat
    Status(CompanionStatus),
    /// AI detection results
    Detections(DetectionFrame),
    /// Log message from companion
    Log { level: String, message: String },
}

/// Commands sent from GCS to companion via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompanionCommand {
    /// Start/stop AI detection
    SetAiEnabled { enabled: bool },
    /// Start/stop video streaming
    SetStreamEnabled { enabled: bool },
    /// Capture a single high-res photo for mapping
    CapturePhoto,
    /// Start/stop survey photo capture mode
    SetSurveyMode { enabled: bool, interval_ms: u32 },
    /// Request companion status
    RequestStatus,
}
