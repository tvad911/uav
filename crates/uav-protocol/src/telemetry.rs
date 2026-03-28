use serde::{Deserialize, Serialize};

use crate::types::{ArmState, Attitude, BatteryStatus, FlightMode, GeoPosition, GpsFixType};

/// Complete telemetry snapshot from the vehicle
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryData {
    /// Timestamp of last update (ms since boot)
    pub timestamp_ms: u64,

    // -- Attitude --
    pub attitude: Attitude,

    // -- Position --
    pub position: GeoPosition,
    pub home_position: Option<GeoPosition>,

    // -- Speed --
    /// Ground speed in m/s
    pub ground_speed: f32,
    /// Airspeed in m/s (from pitot tube)
    pub airspeed: f32,
    /// Vertical speed (climb rate) in m/s
    pub climb_rate: f32,

    // -- Navigation --
    /// Heading in degrees (0-360)
    pub heading: u16,
    /// Distance to home in meters
    pub distance_to_home: f32,
    /// Bearing to home in degrees
    pub bearing_to_home: f32,
    /// Current waypoint index
    pub current_wp: u16,
    /// Distance to next waypoint in meters
    pub distance_to_wp: f32,

    // -- System --
    pub flight_mode: FlightMode,
    pub arm_state: ArmState,
    pub battery: BatteryStatus,
    pub gps_fix: GpsFixType,
    pub gps_satellites: u8,
    pub gps_hdop: f32,

    // -- RC --
    /// RC signal strength (0-255)
    pub rssi: u8,
    /// RC channels (raw PWM values)
    pub rc_channels: Vec<u16>,

    // -- Health --
    /// System status text (last message)
    pub status_text: Option<String>,
    /// Whether heartbeat is being received
    pub heartbeat_ok: bool,
    /// Time since last heartbeat in ms
    pub last_heartbeat_ms: u64,
    
    // -- Session Stats --
    /// Total messages received this session
    pub messages_received: u64,
    /// Flight time since armed (in seconds)
    pub armed_time_s: f32,
}

impl TelemetryData {
    pub fn new() -> Self {
        Self {
            flight_mode: FlightMode::Manual,
            arm_state: ArmState::Disarmed,
            gps_fix: GpsFixType::NoGps,
            ..Default::default()
        }
    }

    /// Calculate distance between current position and home
    pub fn update_home_distance(&mut self) {
        if let Some(home) = &self.home_position {
            self.distance_to_home = haversine_distance(
                self.position.lat,
                self.position.lon,
                home.lat,
                home.lon,
            );
            self.bearing_to_home = bearing(
                self.position.lat,
                self.position.lon,
                home.lat,
                home.lon,
            );
        }
    }
}

/// Haversine distance between two GPS coordinates in meters
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    (R * c) as f32
}

/// Bearing from point 1 to point 2 in degrees (0-360)
fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let x = d_lon.sin() * lat2_r.cos();
    let y = lat1_r.cos() * lat2_r.sin() - lat1_r.sin() * lat2_r.cos() * d_lon.cos();
    let bearing = x.atan2(y).to_degrees();
    ((bearing + 360.0) % 360.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_known_distance() {
        // Hanoi to Ho Chi Minh City ~1,150 km
        let dist = haversine_distance(21.0285, 105.8542, 10.8231, 106.6297);
        assert!((dist - 1_138_000.0).abs() < 20_000.0); // within 20km tolerance
    }

    #[test]
    fn test_bearing_north() {
        let b = bearing(0.0, 0.0, 1.0, 0.0);
        assert!((b - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_bearing_east() {
        let b = bearing(0.0, 0.0, 0.0, 1.0);
        assert!((b - 90.0).abs() < 1.0);
    }
}
