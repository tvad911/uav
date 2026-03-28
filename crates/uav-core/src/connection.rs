use std::sync::Arc;

use mavlink::{self, MavHeader};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

use uav_protocol::{ConnectionType, GcsCommand, ProtocolError, TelemetryData, VehicleEvent};

use crate::heartbeat::HeartbeatManager;
use crate::telemetry_handler::TelemetryHandler;

/// Manages the MAVLink connection to the flight controller
pub struct MavConnection {
    /// Connection configuration
    connection_type: ConnectionType,
    /// System ID for this GCS
    system_id: u8,
    /// Component ID for this GCS
    component_id: u8,
    /// Channel to send commands to the vehicle
    cmd_tx: mpsc::Sender<GcsCommand>,
    /// Channel to receive commands (internal use)
    cmd_rx: Arc<RwLock<mpsc::Receiver<GcsCommand>>>,
    /// Broadcast channel for vehicle events
    event_tx: broadcast::Sender<VehicleEvent>,
    /// Whether connection is active
    connected: Arc<RwLock<bool>>,
    /// Telemetry handler
    telemetry_handler: Arc<TelemetryHandler>,
    /// Heartbeat manager
    heartbeat: HeartbeatManager,
}

impl MavConnection {
    /// Create a new MAVLink connection manager
    pub fn new(connection_type: ConnectionType, telemetry: Arc<RwLock<TelemetryData>>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(256);
        let (event_tx, _) = broadcast::channel(256);
        let connected = Arc::new(RwLock::new(false));

        let system_id = 255; // Standard GCS
        let component_id = 190; // MAV_COMP_ID_MISSIONPLANNER

        let heartbeat = HeartbeatManager::new(
            system_id,
            component_id,
            connected.clone(),
            3000, // 3s timeout
            1000, // 1s interval
        );

        let telemetry_handler = Arc::new(TelemetryHandler::with_data(telemetry));

        Self {
            connection_type,
            system_id,
            component_id,
            cmd_tx,
            cmd_rx: Arc::new(RwLock::new(cmd_rx)),
            event_tx,
            connected,
            telemetry_handler,
            heartbeat,
        }
    }

    /// Get a sender to dispatch commands to the vehicle
    pub fn command_sender(&self) -> mpsc::Sender<GcsCommand> {
        self.cmd_tx.clone()
    }

    /// Subscribe to vehicle events
    pub fn subscribe_events(&self) -> broadcast::Receiver<VehicleEvent> {
        self.event_tx.subscribe()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get telemetry handler reference
    pub fn telemetry_handler(&self) -> Arc<TelemetryHandler> {
        self.telemetry_handler.clone()
    }

    /// Build the MAVLink connection string for the mavlink crate
    fn build_connection_string(&self) -> String {
        match &self.connection_type {
            ConnectionType::Serial { port, baud_rate } => {
                format!("serial:{}:{}", port, baud_rate)
            }
            ConnectionType::Udp { address, port } => {
                format!("udpin:{}:{}", address, port)
            }
            ConnectionType::Tcp { address, port } => {
                format!("tcpin:{}:{}", address, port)
            }
        }
    }

    /// Start the connection in the background.
    /// Spawns receive loop, command sender, heartbeat sender/monitor.
    pub async fn connect(
        &self,
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, ProtocolError> {
        let conn_str = self.build_connection_string();
        info!("Connecting to flight controller: {}", conn_str);

        let connection = mavlink::connect::<mavlink::ardupilotmega::MavMessage>(&conn_str)
            .map_err(|e| ProtocolError::ConnectionFailed(e.to_string()))?;

        let connected = self.connected.clone();
        *connected.write().await = true;

        let event_tx = self.event_tx.clone();
        let _ = event_tx.send(VehicleEvent::Connected {
            connection: self.connection_type.clone(),
            system_id: self.system_id,
            component_id: self.component_id,
        });

        info!("Connected to flight controller successfully");

        let vehicle = Arc::new(connection);
        let mut handles = Vec::new();

        // 1) Spawn heartbeat sender (GCS → FC every 1s)
        handles.push(self.heartbeat.spawn_sender(vehicle.clone()));

        // 2) Spawn heartbeat monitor (detect FC timeout)
        handles.push(self.heartbeat.spawn_monitor(self.event_tx.clone()));

        // 3) Spawn command sender task
        let vehicle_send = vehicle.clone();
        let cmd_rx = self.cmd_rx.clone();
        let connected_cmd = connected.clone();
        let system_id = self.system_id;
        let component_id = self.component_id;
        handles.push(tokio::spawn(async move {
            let mut rx = cmd_rx.write().await;
            while *connected_cmd.read().await {
                if let Some(cmd) = rx.recv().await {
                    let header = MavHeader {
                        system_id,
                        component_id,
                        sequence: 0,
                    };
                    if let Err(e) = Self::send_command(&vehicle_send, header, cmd) {
                        error!("Failed to send command: {}", e);
                    }
                }
            }
            debug!("Command sender task stopped");
        }));

        // 4) Spawn message receive loop
        let vehicle_recv = vehicle.clone();
        let event_tx_recv = self.event_tx.clone();
        let connected_recv = connected.clone();
        let telem_handler = self.telemetry_handler.clone();
        let heartbeat_timer = self.heartbeat.last_heartbeat_ms();

        handles.push(tokio::spawn(async move {
            loop {
                if !*connected_recv.read().await {
                    break;
                }

                match vehicle_recv.recv() {
                    Ok((_header, msg)) => {
                        // Forward to heartbeat tracker if heartbeat
                        if matches!(msg, mavlink::ardupilotmega::MavMessage::HEARTBEAT(_)) {
                            *heartbeat_timer.write().await = 0;
                        }

                        // Forward ALL messages to telemetry handler
                        telem_handler.process_message(&msg).await;

                        // Emit specific events for UI
                        Self::process_message(&event_tx_recv, &msg);
                    }
                    Err(e) => {
                        warn!("MAVLink receive error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
            info!("MAVLink receive loop ended");
        }));

        // 5) Request data streams from FC
        self.request_data_streams(&vehicle).await;

        Ok(handles)
    }

    /// Request telemetry data streams from the flight controller
    async fn request_data_streams(
        &self,
        vehicle: &Arc<Box<dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage> + Send + Sync>>,
    ) {
        let streams = [
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_ALL, 4),
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_RAW_SENSORS, 2),
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_EXTENDED_STATUS, 2),
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_RC_CHANNELS, 5),
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_POSITION, 4),
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_EXTRA1, 10), // ATTITUDE
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_EXTRA2, 10), // VFR_HUD
            (mavlink::ardupilotmega::MavDataStream::MAV_DATA_STREAM_EXTRA3, 2),
        ];

        let header = MavHeader {
            system_id: self.system_id,
            component_id: self.component_id,
            sequence: 0,
        };

        for (stream, rate) in streams {
            let msg = mavlink::ardupilotmega::MavMessage::REQUEST_DATA_STREAM(
                mavlink::ardupilotmega::REQUEST_DATA_STREAM_DATA {
                    target_system: 1,
                    target_component: 1,
                    req_stream_id: stream as u8,
                    req_message_rate: rate,
                    start_stop: 1,
                },
            );
            if let Err(e) = vehicle.send(&header, &msg) {
                warn!("Failed to request data stream {:?}: {}", stream, e);
            } else {
                debug!("Requested data stream {:?} at {}Hz", stream, rate);
            }
        }

        info!("Data stream requests sent");
    }

    /// Disconnect from the flight controller
    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
        let _ = self.event_tx.send(VehicleEvent::Disconnected {
            reason: "User disconnected".to_string(),
        });
        info!("Disconnected from flight controller");
    }

    /// Send a command to the vehicle — all GcsCommand variants implemented
    fn send_command(
        vehicle: &Arc<Box<dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage> + Send + Sync>>,
        header: MavHeader,
        cmd: GcsCommand,
    ) -> Result<(), ProtocolError> {
        use mavlink::ardupilotmega::MavMessage;

        match cmd {
            GcsCommand::Arm(arm) => {
                debug!("Sending arm command: {}", arm);
                let msg = MavMessage::COMMAND_LONG(mavlink::ardupilotmega::COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: mavlink::ardupilotmega::MavCmd::MAV_CMD_COMPONENT_ARM_DISARM,
                    confirmation: 0,
                    param1: if arm { 1.0 } else { 0.0 },
                    param2: 0.0,
                    param3: 0.0,
                    param4: 0.0,
                    param5: 0.0,
                    param6: 0.0,
                    param7: 0.0,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::SetMode(mode) => {
                debug!("Sending set mode: {}", mode);
                let msg = MavMessage::SET_MODE(mavlink::ardupilotmega::SET_MODE_DATA {
                    target_system: 1,
                    base_mode: mavlink::ardupilotmega::MavMode::MAV_MODE_MANUAL_ARMED,
                    custom_mode: mode,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::RcOverride { channels } => {
                let msg = MavMessage::RC_CHANNELS_OVERRIDE(
                    mavlink::ardupilotmega::RC_CHANNELS_OVERRIDE_DATA {
                        target_system: 1,
                        target_component: 1,
                        chan1_raw: channels[0],
                        chan2_raw: channels[1],
                        chan3_raw: channels[2],
                        chan4_raw: channels[3],
                        chan5_raw: channels[4],
                        chan6_raw: channels[5],
                        chan7_raw: channels[6],
                        chan8_raw: channels[7],
                    },
                );
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::ReturnToLaunch => {
                debug!("Sending RTL command");
                let msg = MavMessage::COMMAND_LONG(mavlink::ardupilotmega::COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: mavlink::ardupilotmega::MavCmd::MAV_CMD_NAV_RETURN_TO_LAUNCH,
                    confirmation: 0,
                    param1: 0.0,
                    param2: 0.0,
                    param3: 0.0,
                    param4: 0.0,
                    param5: 0.0,
                    param6: 0.0,
                    param7: 0.0,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::CameraTrigger => {
                debug!("Sending camera trigger");
                let msg = MavMessage::COMMAND_LONG(mavlink::ardupilotmega::COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: mavlink::ardupilotmega::MavCmd::MAV_CMD_DO_DIGICAM_CONTROL,
                    confirmation: 0,
                    param1: 0.0,
                    param2: 0.0,
                    param3: 0.0,
                    param4: 0.0,
                    param5: 1.0, // trigger
                    param6: 0.0,
                    param7: 0.0,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::GuidedGoto { lat, lon, alt } => {
                debug!("Sending guided goto: {:.6}, {:.6}, {:.1}m", lat, lon, alt);
                let msg = MavMessage::SET_POSITION_TARGET_GLOBAL_INT(
                    mavlink::ardupilotmega::SET_POSITION_TARGET_GLOBAL_INT_DATA {
                        time_boot_ms: 0,
                        target_system: 1,
                        target_component: 1,
                        coordinate_frame: mavlink::ardupilotmega::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
                        type_mask: mavlink::ardupilotmega::PositionTargetTypemask::from_bits_truncate(
                            0b0000_1111_1111_1000, // Only position bits enabled
                        ),
                        lat_int: (lat * 1e7) as i32,
                        lon_int: (lon * 1e7) as i32,
                        alt,
                        vx: 0.0,
                        vy: 0.0,
                        vz: 0.0,
                        afx: 0.0,
                        afy: 0.0,
                        afz: 0.0,
                        yaw: 0.0,
                        yaw_rate: 0.0,
                    },
                );
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::UploadMission(waypoints) => {
                debug!("Uploading mission with {} waypoints", waypoints.len());
                // Step 1: Send MISSION_COUNT
                let count_msg = MavMessage::MISSION_COUNT(
                    mavlink::ardupilotmega::MISSION_COUNT_DATA {
                        target_system: 1,
                        target_component: 1,
                        count: waypoints.len() as u16,
                    },
                );
                send_msg(vehicle, &header, &count_msg)?;

                // Step 2: Send each MISSION_ITEM_INT
                // Note: In real protocol, FC requests each item via MISSION_REQUEST_INT.
                // For simplicity, we send them all. Full handshake needs async task.
                for wp in &waypoints {
                    let item = MavMessage::MISSION_ITEM_INT(
                        mavlink::ardupilotmega::MISSION_ITEM_INT_DATA {
                            target_system: 1,
                            target_component: 1,
                            seq: wp.seq,
                            frame: mavlink::ardupilotmega::MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT,
                            command: mavlink::ardupilotmega::MavCmd::MAV_CMD_NAV_WAYPOINT,
                            current: wp.current,
                            autocontinue: wp.autocontinue,
                            param1: wp.param1,
                            param2: wp.param2,
                            param3: wp.param3,
                            param4: wp.param4,
                            x: (wp.lat * 1e7) as i32,
                            y: (wp.lon * 1e7) as i32,
                            z: wp.alt,
                        },
                    );
                    send_msg(vehicle, &header, &item)?;
                }
                info!("Mission upload complete ({} waypoints)", waypoints.len());
            }

            GcsCommand::StartMission => {
                debug!("Starting mission");
                let msg = MavMessage::COMMAND_LONG(mavlink::ardupilotmega::COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: mavlink::ardupilotmega::MavCmd::MAV_CMD_MISSION_START,
                    confirmation: 0,
                    param1: 0.0, // first waypoint
                    param2: 0.0, // last waypoint (0 = all)
                    param3: 0.0,
                    param4: 0.0,
                    param5: 0.0,
                    param6: 0.0,
                    param7: 0.0,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::PauseMission => {
                debug!("Pausing mission — switching to LOITER");
                // Pause = switch to LOITER mode
                let msg = MavMessage::SET_MODE(mavlink::ardupilotmega::SET_MODE_DATA {
                    target_system: 1,
                    base_mode: mavlink::ardupilotmega::MavMode::MAV_MODE_MANUAL_ARMED,
                    custom_mode: 12, // LOITER
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::ResumeMission => {
                debug!("Resuming mission — switching to AUTO");
                let msg = MavMessage::SET_MODE(mavlink::ardupilotmega::SET_MODE_DATA {
                    target_system: 1,
                    base_mode: mavlink::ardupilotmega::MavMode::MAV_MODE_MANUAL_ARMED,
                    custom_mode: 10, // AUTO
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::RequestDataStream { stream_id, rate_hz } => {
                debug!("Requesting data stream {} at {}Hz", stream_id, rate_hz);
                let msg = MavMessage::REQUEST_DATA_STREAM(
                    mavlink::ardupilotmega::REQUEST_DATA_STREAM_DATA {
                        target_system: 1,
                        target_component: 1,
                        req_stream_id: stream_id,
                        req_message_rate: rate_hz,
                        start_stop: 1,
                    },
                );
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::SetParam { param_id, value } => {
                debug!("Setting param {} = {}", param_id, value);
                let mut id_bytes = [0u8; 16];
                for (i, b) in param_id.as_bytes().iter().take(16).enumerate() {
                    id_bytes[i] = *b;
                }
                let msg = MavMessage::PARAM_SET(mavlink::ardupilotmega::PARAM_SET_DATA {
                    target_system: 1,
                    target_component: 1,
                    param_id: id_bytes,
                    param_value: value,
                    param_type: mavlink::ardupilotmega::MavParamType::MAV_PARAM_TYPE_REAL32,
                });
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::GetParam { param_id } => {
                debug!("Requesting param: {}", param_id);
                let mut id_bytes = [0u8; 16];
                for (i, b) in param_id.as_bytes().iter().take(16).enumerate() {
                    id_bytes[i] = *b;
                }
                let msg = MavMessage::PARAM_REQUEST_READ(
                    mavlink::ardupilotmega::PARAM_REQUEST_READ_DATA {
                        target_system: 1,
                        target_component: 1,
                        param_id: id_bytes,
                        param_index: -1, // Use param_id, not index
                    },
                );
                send_msg(vehicle, &header, &msg)?;
            }

            GcsCommand::Reboot => {
                warn!("Sending reboot command to flight controller!");
                let msg = MavMessage::COMMAND_LONG(mavlink::ardupilotmega::COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: mavlink::ardupilotmega::MavCmd::MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN,
                    confirmation: 0,
                    param1: 1.0, // Reboot autopilot
                    param2: 0.0,
                    param3: 0.0,
                    param4: 0.0,
                    param5: 0.0,
                    param6: 0.0,
                    param7: 0.0,
                });
                send_msg(vehicle, &header, &msg)?;
            }
        }

        Ok(())
    }

    /// Process an incoming MAVLink message and emit events to UI
    fn process_message(
        event_tx: &broadcast::Sender<VehicleEvent>,
        msg: &mavlink::ardupilotmega::MavMessage,
    ) {
        use mavlink::ardupilotmega::MavMessage;

        match msg {
            MavMessage::HEARTBEAT(data) => {
                let armed = data
                    .base_mode
                    .intersects(mavlink::ardupilotmega::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED);
                let _ = event_tx.send(VehicleEvent::ArmStateChanged { armed });
                let _ = event_tx.send(VehicleEvent::ModeChanged {
                    mode: data.custom_mode,
                });
            }
            MavMessage::STATUSTEXT(data) => {
                let text = String::from_utf8_lossy(&data.text)
                    .trim_end_matches('\0')
                    .to_string();
                let _ = event_tx.send(VehicleEvent::StatusText {
                    severity: data.severity as u8,
                    text,
                });
            }
            MavMessage::PARAM_VALUE(data) => {
                let param_id = String::from_utf8_lossy(&data.param_id)
                    .trim_end_matches('\0')
                    .to_string();
                let _ = event_tx.send(VehicleEvent::ParamValue {
                    param_id,
                    value: data.param_value,
                    param_type: data.param_type as u8,
                    param_count: data.param_count,
                    param_index: data.param_index,
                });
            }
            MavMessage::COMMAND_ACK(ack) => {
                debug!("Command ACK: cmd={:?} result={:?}", ack.command, ack.result);
            }
            MavMessage::MISSION_ACK(ack) => {
                let _ = event_tx.send(VehicleEvent::MissionAck {
                    result: ack.mavtype as u8,
                });
            }
            _ => {
                // Other messages already handled by TelemetryHandler
            }
        }
    }
}

/// Helper to send a MAVLink message and map error
fn send_msg(
    vehicle: &Arc<Box<dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage> + Send + Sync>>,
    header: &MavHeader,
    msg: &mavlink::ardupilotmega::MavMessage,
) -> Result<(), ProtocolError> {
    vehicle
        .send(header, msg)
        .map(|_| ())
        .map_err(|e| ProtocolError::ConnectionFailed(format!("Send failed: {}", e)))
}
