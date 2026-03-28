use serde::{Deserialize, Serialize};

/// Flight mode definitions for ArduPlane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FlightMode {
    #[default]
    Manual,
    Stabilize,
    FlyByWireA,
    FlyByWireB,
    Auto,
    Rtl,
    Loiter,
    Cruise,
    Guided,
    Circle,
    Unknown(u32),
}

impl FlightMode {
    /// Convert ArduPlane custom mode number to FlightMode
    pub fn from_custom_mode(mode: u32) -> Self {
        match mode {
            0 => Self::Manual,
            2 => Self::Stabilize,
            5 => Self::FlyByWireA,
            6 => Self::FlyByWireB,
            10 => Self::Auto,
            11 => Self::Rtl,
            12 => Self::Loiter,
            7 => Self::Cruise,
            15 => Self::Guided,
            1 => Self::Circle,
            other => Self::Unknown(other),
        }
    }

    /// Convert FlightMode to ArduPlane custom mode number
    pub fn to_custom_mode(self) -> u32 {
        match self {
            Self::Manual => 0,
            Self::Stabilize => 2,
            Self::FlyByWireA => 5,
            Self::FlyByWireB => 6,
            Self::Auto => 10,
            Self::Rtl => 11,
            Self::Loiter => 12,
            Self::Cruise => 7,
            Self::Guided => 15,
            Self::Circle => 1,
            Self::Unknown(m) => m,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Manual => "MANUAL",
            Self::Stabilize => "STABILIZE",
            Self::FlyByWireA => "FBWA",
            Self::FlyByWireB => "FBWB",
            Self::Auto => "AUTO",
            Self::Rtl => "RTL",
            Self::Loiter => "LOITER",
            Self::Cruise => "CRUISE",
            Self::Guided => "GUIDED",
            Self::Circle => "CIRCLE",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

/// GPS fix type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GpsFixType {
    #[default]
    NoGps,
    NoFix,
    Fix2D,
    Fix3D,
    DGps,
    RtkFloat,
    RtkFixed,
    Unknown(u8),
}

impl From<u8> for GpsFixType {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::NoGps,
            1 => Self::NoFix,
            2 => Self::Fix2D,
            3 => Self::Fix3D,
            4 => Self::DGps,
            5 => Self::RtkFloat,
            6 => Self::RtkFixed,
            other => Self::Unknown(other),
        }
    }
}

/// Arm state of the vehicle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ArmState {
    #[default]
    Disarmed,
    Armed,
}

/// Geographic coordinate
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct GeoPosition {
    /// Latitude in degrees
    pub lat: f64,
    /// Longitude in degrees
    pub lon: f64,
    /// Altitude in meters (above sea level)
    pub alt_msl: f32,
    /// Altitude in meters (above ground level)
    pub alt_agl: f32,
}

/// 3D attitude (Euler angles)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Attitude {
    /// Roll angle in degrees (-180 to 180)
    pub roll: f32,
    /// Pitch angle in degrees (-90 to 90)
    pub pitch: f32,
    /// Yaw/heading in degrees (0 to 360)
    pub yaw: f32,
    /// Roll rate in deg/s
    pub roll_speed: f32,
    /// Pitch rate in deg/s
    pub pitch_speed: f32,
    /// Yaw rate in deg/s
    pub yaw_speed: f32,
}

/// Battery information
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BatteryStatus {
    /// Voltage in volts
    pub voltage: f32,
    /// Current draw in amps
    pub current: f32,
    /// Remaining capacity in percent (0-100)
    pub remaining_pct: i8,
    /// mAh consumed
    pub mah_consumed: u32,
}

/// Waypoint for mission planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub seq: u16,
    pub frame: u8,
    pub command: u16,
    pub current: u8,
    pub autocontinue: u8,
    pub param1: f32,
    pub param2: f32,
    pub param3: f32,
    pub param4: f32,
    pub lat: f64,
    pub lon: f64,
    pub alt: f32,
}

/// Connection type to the flight controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Serial port (USB or telemetry radio)
    Serial {
        port: String,
        baud_rate: u32,
    },
    /// UDP connection (SITL or network telemetry)
    Udp {
        address: String,
        port: u16,
    },
    /// TCP connection
    Tcp {
        address: String,
        port: u16,
    },
}

impl Default for ConnectionType {
    fn default() -> Self {
        // Default to SITL UDP
        Self::Udp {
            address: "127.0.0.1".to_string(),
            port: 14550,
        }
    }
}
