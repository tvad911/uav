use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, warn};

use uav_protocol::{
    ArmState, Attitude, BatteryStatus, FlightMode, GeoPosition, GpsFixType, TelemetryData,
};

/// Processes incoming MAVLink messages and updates telemetry state
pub struct TelemetryHandler {
    /// Shared telemetry data
    data: Arc<RwLock<TelemetryData>>,
}

impl TelemetryHandler {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(TelemetryData::new())),
        }
    }

    /// Create with a pre-existing shared telemetry data reference
    pub fn with_data(data: Arc<RwLock<TelemetryData>>) -> Self {
        Self {
            data,
        }
    }

    /// Get a shared reference to the telemetry data (for the UI to read)
    pub fn data(&self) -> Arc<RwLock<TelemetryData>> {
        self.data.clone()
    }

    /// Process a raw MAVLink message and update telemetry
    pub async fn process_message(&self, msg: &mavlink::ardupilotmega::MavMessage) {
        use mavlink::ardupilotmega::MavMessage;

        let mut data = self.data.write().await;
        data.messages_received += 1;

        match msg {
            MavMessage::HEARTBEAT(hb) => {
                data.heartbeat_ok = true;
                data.last_heartbeat_ms = 0;
                data.flight_mode = FlightMode::from_custom_mode(hb.custom_mode);
                data.arm_state = if hb.base_mode.intersects(
                    mavlink::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED,
                ) {
                    ArmState::Armed
                } else {
                    ArmState::Disarmed
                };
            }

            MavMessage::ATTITUDE(att) => {
                data.attitude = Attitude {
                    roll: att.roll.to_degrees(),
                    pitch: att.pitch.to_degrees(),
                    yaw: att.yaw.to_degrees(),
                    roll_speed: att.rollspeed.to_degrees(),
                    pitch_speed: att.pitchspeed.to_degrees(),
                    yaw_speed: att.yawspeed.to_degrees(),
                };
            }

            MavMessage::GLOBAL_POSITION_INT(pos) => {
                data.position = GeoPosition {
                    lat: pos.lat as f64 / 1e7,
                    lon: pos.lon as f64 / 1e7,
                    alt_msl: pos.alt as f32 / 1000.0,
                    alt_agl: pos.relative_alt as f32 / 1000.0,
                };
                data.heading = pos.hdg / 100;
                data.climb_rate = pos.vz as f32 / -100.0; // Positive = up
                data.update_home_distance();
            }

            MavMessage::GPS_RAW_INT(gps) => {
                data.gps_fix = GpsFixType::from(gps.fix_type as u8);
                data.gps_satellites = gps.satellites_visible;
                data.gps_hdop = gps.eph as f32 / 100.0;
            }

            MavMessage::VFR_HUD(hud) => {
                data.airspeed = hud.airspeed;
                data.ground_speed = hud.groundspeed;
                data.heading = hud.heading as u16;
                data.climb_rate = hud.climb;
            }

            MavMessage::SYS_STATUS(sys) => {
                data.battery = BatteryStatus {
                    voltage: sys.voltage_battery as f32 / 1000.0,
                    current: sys.current_battery as f32 / 100.0,
                    remaining_pct: sys.battery_remaining,
                    mah_consumed: 0, // Not available in SYS_STATUS
                };
            }

            MavMessage::BATTERY_STATUS(batt) => {
                data.battery.mah_consumed = batt.current_consumed as u32;
                if batt.battery_remaining >= 0 {
                    data.battery.remaining_pct = batt.battery_remaining;
                }
            }

            MavMessage::RC_CHANNELS(rc) => {
                data.rssi = rc.rssi;
                data.rc_channels = vec![
                    rc.chan1_raw,
                    rc.chan2_raw,
                    rc.chan3_raw,
                    rc.chan4_raw,
                    rc.chan5_raw,
                    rc.chan6_raw,
                    rc.chan7_raw,
                    rc.chan8_raw,
                ];
            }

            MavMessage::HOME_POSITION(home) => {
                data.home_position = Some(GeoPosition {
                    lat: home.latitude as f64 / 1e7,
                    lon: home.longitude as f64 / 1e7,
                    alt_msl: home.altitude as f32 / 1000.0,
                    alt_agl: 0.0,
                });
                debug!(
                    "Home position set: {:.6}, {:.6}",
                    home.latitude as f64 / 1e7,
                    home.longitude as f64 / 1e7
                );
            }

            MavMessage::MISSION_CURRENT(mc) => {
                data.current_wp = mc.seq;
            }

            MavMessage::NAV_CONTROLLER_OUTPUT(nav) => {
                data.distance_to_wp = nav.wp_dist as f32;
            }

            MavMessage::STATUSTEXT(st) => {
                let text = String::from_utf8_lossy(&st.text)
                    .trim_end_matches('\0')
                    .to_string();
                if !text.is_empty() {
                    data.status_text = Some(text);
                }
            }

            _ => {
                // Ignore messages we don't handle
            }
        }
    }

    /// Update heartbeat timing - call periodically to detect connection loss
    pub async fn tick_heartbeat(&self, elapsed_ms: u64) {
        let mut data = self.data.write().await;
        data.last_heartbeat_ms += elapsed_ms;
        if data.last_heartbeat_ms > 3000 {
            if data.heartbeat_ok {
                warn!("Heartbeat lost! Last seen {}ms ago", data.last_heartbeat_ms);
            }
            data.heartbeat_ok = false;
        }
    }
}

impl Default for TelemetryHandler {
    fn default() -> Self {
        Self::new()
    }
}
