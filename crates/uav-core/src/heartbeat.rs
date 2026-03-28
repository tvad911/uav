use std::sync::Arc;
use std::time::Duration;

use mavlink::MavHeader;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use uav_protocol::VehicleEvent;

/// Manages GCS heartbeat sending and FC heartbeat monitoring
pub struct HeartbeatManager {
    /// GCS system ID
    system_id: u8,
    /// GCS component ID
    component_id: u8,
    /// Whether connection is active
    connected: Arc<RwLock<bool>>,
    /// Time since last FC heartbeat in ms
    last_fc_heartbeat_ms: Arc<RwLock<u64>>,
    /// Heartbeat timeout threshold in ms
    timeout_ms: u64,
    /// Heartbeat send interval in ms
    interval_ms: u64,
}

impl HeartbeatManager {
    pub fn new(
        system_id: u8,
        component_id: u8,
        connected: Arc<RwLock<bool>>,
        timeout_ms: u64,
        interval_ms: u64,
    ) -> Self {
        Self {
            system_id,
            component_id,
            connected,
            last_fc_heartbeat_ms: Arc::new(RwLock::new(0)),
            timeout_ms,
            interval_ms,
        }
    }

    /// Record that a heartbeat was received from the FC
    pub async fn on_heartbeat_received(&self) {
        *self.last_fc_heartbeat_ms.write().await = 0;
    }

    /// Get the shared last heartbeat timer for external monitoring
    pub fn last_heartbeat_ms(&self) -> Arc<RwLock<u64>> {
        self.last_fc_heartbeat_ms.clone()
    }

    /// Spawn the heartbeat sender task.
    /// Sends GCS heartbeat to FC at configured interval.
    pub fn spawn_sender(
        &self,
        vehicle: Arc<Box<dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage> + Send + Sync>>,
    ) -> tokio::task::JoinHandle<()> {
        let system_id = self.system_id;
        let component_id = self.component_id;
        let connected = self.connected.clone();
        let interval = Duration::from_millis(self.interval_ms);

        tokio::spawn(async move {
            let mut seq: u8 = 0;
            loop {
                if !*connected.read().await {
                    break;
                }

                let header = MavHeader {
                    system_id,
                    component_id,
                    sequence: seq,
                };

                let heartbeat = mavlink::ardupilotmega::MavMessage::HEARTBEAT(
                    mavlink::ardupilotmega::HEARTBEAT_DATA {
                        custom_mode: 0,
                        mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_GCS,
                        autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
                        base_mode: mavlink::ardupilotmega::MavModeFlag::default(),
                        system_status: mavlink::ardupilotmega::MavState::MAV_STATE_ACTIVE,
                        mavlink_version: 3,
                    },
                );

                if let Err(e) = vehicle.send(&header, &heartbeat) {
                    warn!("Failed to send GCS heartbeat: {}", e);
                } else {
                    debug!("GCS heartbeat sent (seq={})", seq);
                }

                seq = seq.wrapping_add(1);
                tokio::time::sleep(interval).await;
            }
            debug!("Heartbeat sender stopped");
        })
    }

    /// Spawn the heartbeat monitor task.
    /// Tracks time since last FC heartbeat and emits timeout events.
    pub fn spawn_monitor(
        &self,
        event_tx: tokio::sync::broadcast::Sender<VehicleEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let connected = self.connected.clone();
        let last_hb = self.last_fc_heartbeat_ms.clone();
        let timeout_ms = self.timeout_ms;
        let tick_interval = Duration::from_millis(500);

        tokio::spawn(async move {
            let mut was_timed_out = false;

            loop {
                if !*connected.read().await {
                    break;
                }

                tokio::time::sleep(tick_interval).await;

                let mut ms = last_hb.write().await;
                *ms += 500;

                if *ms > timeout_ms {
                    if !was_timed_out {
                        warn!("FC heartbeat timeout! Last seen {}ms ago", *ms);
                        let _ = event_tx.send(VehicleEvent::HeartbeatTimeout {
                            last_seen_ms: *ms,
                        });
                        was_timed_out = true;
                    }
                } else {
                    was_timed_out = false;
                }
            }
            debug!("Heartbeat monitor stopped");
        })
    }
}
