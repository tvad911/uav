use std::sync::Arc;

use eframe::egui;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use uav_core::AppConfig;
use uav_protocol::{
    BackendEvent, CompanionStatus, ConnectionType, DetectionFrame, GcsCommand, TelemetryData,
    UiCommand,
};

use crate::panels::attitude_indicator::AttitudeIndicator;
use crate::panels::companion_panel::CompanionPanel;
// Will be used when WebSocket companion link is implemented
#[allow(unused_imports)]
use crate::panels::detection_overlay::DetectionOverlay;
use crate::panels::map_view::MapView;
use crate::panels::status_bar::StatusBar;
use crate::panels::telemetry_panel::TelemetryPanel;
use crate::panels::video_view::VideoView;
use walkers::{HttpTiles, MapMemory, sources::OpenStreetMap};

/// Main GCS application state
pub struct GcsApp {
    /// Shared telemetry data
    telemetry: Arc<RwLock<TelemetryData>>,
    /// Application config
    #[allow(dead_code)]
    config: AppConfig,
    /// Current telemetry snapshot (for UI thread)
    telem_snapshot: TelemetryData,
    /// Connection status
    connected: bool,
    /// Connection string input
    connection_input: String,
    /// Log messages
    log_messages: Vec<LogEntry>,
    /// Channel to send commands to the backend
    ui_cmd_tx: mpsc::Sender<UiCommand>,
    /// Channel to receive events from the backend
    backend_evt_rx: mpsc::Receiver<BackendEvent>,
    
    // -- Stats tracking --
    last_message_count: u64,
    last_calc_time: f64,
    pub data_rate_hz: f32,

    // -- Map state --
    map_memory: MapMemory,
    map_tiles: HttpTiles,

    // -- Video state --
    video_receiver: Option<uav_video::VideoReceiver>,
    video_texture: Option<egui::TextureHandle>,

    // -- Companion Computer state --
    companion_status: CompanionStatus,
    #[allow(dead_code)]
    detection_frame: DetectionFrame,
}

#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    severity: LogSeverity,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum LogSeverity {
    Info,
    Warning,
    Error,
}

impl GcsApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        telemetry: Arc<RwLock<TelemetryData>>,
        config: AppConfig,
        ui_cmd_tx: mpsc::Sender<UiCommand>,
        backend_evt_rx: mpsc::Receiver<BackendEvent>,
    ) -> Self {
        let mut app = Self {
            telemetry,
            config,
            telem_snapshot: TelemetryData::new(),
            connected: false,
            connection_input: "udpin:127.0.0.1:14550".to_string(),
            log_messages: vec![LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                severity: LogSeverity::Info,
                message: "UAV Ground Control Station initialized".to_string(),
            }],
            ui_cmd_tx,
            backend_evt_rx,
            last_message_count: 0,
            last_calc_time: 0.0,
            data_rate_hz: 0.0,

            // Initialize map
            map_memory: MapMemory::default(),
            map_tiles: HttpTiles::new(OpenStreetMap, _cc.egui_ctx.clone()),
            
            // Initialize video
            video_receiver: None,
            video_texture: None,

            // Initialize companion
            companion_status: CompanionStatus::default(),
            detection_frame: DetectionFrame::default(),
        };

        // Attempt to start UDP video receiver immediately
        match uav_video::VideoReceiver::new(5600) {
            Ok(vr) => {
                if let Err(e) = vr.start() {
                    tracing::error!("Failed to start VideoReceiver: {:?}", e);
                } else {
                    tracing::info!("Started VideoReceiver on UDP 5600");
                    app.video_receiver = Some(vr);
                }
            }
            Err(e) => {
                tracing::error!("Failed to init VideoReceiver: {:?}", e);
            }
        }
        
        app
    }

    fn add_log(&mut self, severity: LogSeverity, message: String) {
        self.log_messages.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            severity,
            message,
        });
        // Keep last 200 messages
        if self.log_messages.len() > 200 {
            self.log_messages.remove(0);
        }
    }

    /// Parse connection string → ConnectionType
    fn parse_connection_string(input: &str) -> Option<ConnectionType> {
        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        match parts[0] {
            "serial" => {
                let baud: u32 = parts.get(2)?.parse().ok()?;
                Some(ConnectionType::Serial {
                    port: parts[1].to_string(),
                    baud_rate: baud,
                })
            }
            "udpin" | "udp" => {
                let port: u16 = parts.get(2)?.parse().ok()?;
                Some(ConnectionType::Udp {
                    address: parts[1].to_string(),
                    port,
                })
            }
            "tcpin" | "tcp" => {
                let port: u16 = parts.get(2)?.parse().ok()?;
                Some(ConnectionType::Tcp {
                    address: parts[1].to_string(),
                    port,
                })
            }
            _ => None,
        }
    }

    /// Send a command to the backend (non-blocking)
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(e) = self.ui_cmd_tx.try_send(cmd) {
            tracing::error!("Failed to send UI command: {}", e);
        }
    }

    /// Send a GCS command to the vehicle
    fn send_vehicle_command(&self, cmd: GcsCommand) {
        self.send_ui_command(UiCommand::SendCommand(cmd));
    }

    /// Poll backend events (non-blocking)
    fn poll_backend_events(&mut self) {
        while let Ok(evt) = self.backend_evt_rx.try_recv() {
            match evt {
                BackendEvent::Connected => {
                    self.connected = true;
                    self.add_log(LogSeverity::Info, "Connected to flight controller".to_string());
                    info!("Backend reports: Connected");
                }
                BackendEvent::Disconnected(reason) => {
                    self.connected = false;
                    self.add_log(LogSeverity::Info, format!("Disconnected: {}", reason));
                }
                BackendEvent::Error(msg) => {
                    self.add_log(LogSeverity::Error, msg);
                }
                BackendEvent::TelemetryUpdated => {
                    // Handled via shared state
                }
                BackendEvent::Vehicle(event) => {
                    use uav_protocol::VehicleEvent;
                    match event {
                        VehicleEvent::StatusText { severity, text } => {
                            let sev = if severity <= 3 {
                                LogSeverity::Error
                            } else if severity <= 5 {
                                LogSeverity::Warning
                            } else {
                                LogSeverity::Info
                            };
                            self.add_log(sev, format!("[FC] {}", text));
                        }
                        VehicleEvent::HeartbeatTimeout { last_seen_ms } => {
                            self.add_log(
                                LogSeverity::Warning,
                                format!("Heartbeat timeout! Last: {}ms ago", last_seen_ms),
                            );
                        }
                        VehicleEvent::Failsafe { reason } => {
                            self.add_log(
                                LogSeverity::Error,
                                format!("FAILSAFE: {:?}", reason),
                            );
                        }
                        VehicleEvent::MissionAck { result } => {
                            self.add_log(
                                LogSeverity::Info,
                                format!("Mission ACK: result={}", result),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl eframe::App for GcsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_time = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.stable_dt);
        
        // Poll backend events
        self.poll_backend_events();

        // Update telemetry snapshot from shared state (non-blocking)
        if let Ok(mut data) = self.telemetry.try_write() {
            // Update flight time if armed
            if data.arm_state == uav_protocol::ArmState::Armed {
                data.armed_time_s += dt;
            } else {
                data.armed_time_s = 0.0;
            }
            
            self.telem_snapshot = data.clone();
        }

        // Poll video receiver
        if let Some(vr) = &self.video_receiver {
            if let Some(frame) = vr.take_latest_frame() {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.data,
                );
                
                // Update texture or create new
                if let Some(tex) = &mut self.video_texture {
                    tex.set_partial([0, 0], image, Default::default());
                } else {
                    self.video_texture = Some(ctx.load_texture("video_feed", image, Default::default()));
                }
            }
        }

        // Calculate data rate every second
        if current_time - self.last_calc_time > 1.0 {
            let received = self.telem_snapshot.messages_received;
            let diff = received.saturating_sub(self.last_message_count);
            self.data_rate_hz = diff as f32 / (current_time - self.last_calc_time) as f32;
            self.last_message_count = received;
            self.last_calc_time = current_time;
        }

        // Request repaint for live telemetry updates (~20fps)
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        // Setup dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(18, 18, 24);
        visuals.window_fill = egui::Color32::from_rgb(24, 24, 32);
        ctx.set_visuals(visuals);

        // -- Top menu bar --
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.label(
                    egui::RichText::new("✈ UAV GCS")
                        .strong()
                        .size(16.0)
                        .color(egui::Color32::from_rgb(100, 180, 255)),
                );
                ui.separator();

                // Connection controls
                ui.label("Connection:");
                let text_edit = egui::TextEdit::singleline(&mut self.connection_input)
                    .desired_width(220.0);
                ui.add(text_edit);

                if self.connected {
                    if ui
                        .button(egui::RichText::new("⏹ Disconnect").color(egui::Color32::RED))
                        .clicked()
                    {
                        self.send_ui_command(UiCommand::Disconnect);
                    }
                } else if ui
                    .button(
                        egui::RichText::new("▶ Connect").color(egui::Color32::GREEN),
                    )
                    .clicked()
                {
                    match Self::parse_connection_string(&self.connection_input) {
                        Some(conn_type) => {
                            self.add_log(
                                LogSeverity::Info,
                                format!("Connecting to {:?}...", conn_type),
                            );
                            self.send_ui_command(UiCommand::Connect(conn_type));
                        }
                        None => {
                            self.add_log(
                                LogSeverity::Error,
                                format!("Invalid connection string: {}", self.connection_input),
                            );
                        }
                    }
                }
            });
        });

        // -- Bottom status bar --
        StatusBar::show(ctx, &self.telem_snapshot, self.connected, self.data_rate_hz);

        // -- Left panel: Telemetry gauges --
        egui::SidePanel::left("telemetry_panel")
            .default_width(280.0)
            .show(ctx, |ui| {
                TelemetryPanel::show(ui, &self.telem_snapshot, self.data_rate_hz);
            });

        // -- Right panel: Controls & Log --
        egui::SidePanel::right("control_panel")
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Controls");
                ui.separator();

                // Flight mode buttons
                ui.horizontal_wrapped(|ui| {
                    let modes: [(& str, u32); 8] = [
                        ("MANUAL", 0),
                        ("STAB", 2),
                        ("FBWA", 5),
                        ("FBWB", 6),
                        ("AUTO", 10),
                        ("RTL", 11),
                        ("LOITER", 12),
                        ("CRUISE", 7),
                    ];
                    for (name, mode_id) in modes {
                        let is_current = self.telem_snapshot.flight_mode.name() == name;
                        if ui
                            .selectable_label(is_current, name)
                            .clicked()
                        {
                            self.add_log(
                                LogSeverity::Info,
                                format!("Mode change → {}", name),
                            );
                            self.send_vehicle_command(GcsCommand::SetMode(mode_id));
                        }
                    }
                });

                ui.separator();

                // Arm / Disarm / RTL
                ui.horizontal(|ui| {
                    let armed = self.telem_snapshot.arm_state == uav_protocol::ArmState::Armed;
                    if armed {
                        if ui
                            .button(
                                egui::RichText::new("🔓 DISARM")
                                    .color(egui::Color32::YELLOW)
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.add_log(LogSeverity::Warning, "Disarm requested".to_string());
                            self.send_vehicle_command(GcsCommand::Arm(false));
                        }
                    } else if ui
                        .button(
                            egui::RichText::new("🔒 ARM")
                                .color(egui::Color32::GREEN)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.add_log(LogSeverity::Warning, "Arm requested".to_string());
                        self.send_vehicle_command(GcsCommand::Arm(true));
                    }

                    if ui.button("🏠 RTL").clicked() {
                        self.add_log(
                            LogSeverity::Warning,
                            "Return to Launch requested".to_string(),
                        );
                        self.send_vehicle_command(GcsCommand::ReturnToLaunch);
                    }
                });

                ui.separator();

                // Mission controls
                ui.horizontal(|ui| {
                    if ui.button("▶ Start Mission").clicked() {
                        self.add_log(LogSeverity::Info, "Start mission requested".to_string());
                        self.send_vehicle_command(GcsCommand::StartMission);
                    }
                    if ui.button("⏸ Pause").clicked() {
                        self.add_log(LogSeverity::Info, "Pause mission".to_string());
                        self.send_vehicle_command(GcsCommand::PauseMission);
                    }
                    if ui.button("⏵ Resume").clicked() {
                        self.add_log(LogSeverity::Info, "Resume mission".to_string());
                        self.send_vehicle_command(GcsCommand::ResumeMission);
                    }
                });

                ui.separator();

                // Camera trigger
                ui.horizontal(|ui| {
                    if ui.button("📷 Camera").clicked() {
                        self.add_log(LogSeverity::Info, "Camera trigger".to_string());
                        self.send_vehicle_command(GcsCommand::CameraTrigger);
                    }
                });

                ui.separator();
                ui.heading("Messages");

                // Log area
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for entry in &self.log_messages {
                            let color = match entry.severity {
                                LogSeverity::Info => egui::Color32::from_rgb(150, 150, 150),
                                LogSeverity::Warning => egui::Color32::YELLOW,
                                LogSeverity::Error => egui::Color32::RED,
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&entry.timestamp)
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.label(egui::RichText::new(&entry.message).color(color));
                            });
                        }
                    });
            });

        // -- Right panel: Companion Computer Status --
        egui::SidePanel::right("companion_panel")
            .default_width(220.0)
            .min_width(180.0)
            .show(ctx, |ui| {
                CompanionPanel::show(ui, &self.companion_status);
            });

        // -- Central panel: Attitude indicator + Map/Video feed --
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_height = ui.available_height();

            // Top half: Attitude indicator
            let attitude_height = available_height * 0.4;
            ui.allocate_ui(
                egui::Vec2::new(ui.available_width(), attitude_height),
                |ui| {
                    AttitudeIndicator::show(ui, &self.telem_snapshot);
                },
            );

            ui.separator();

            // Bottom half: Map feed and Video feed (side by side inside a Columns or Horizontal)
            ui.horizontal(|ui| {
                // Let's divide equally
                let width = ui.available_width() * 0.5;
                
                ui.allocate_ui(egui::Vec2::new(width, ui.available_height()), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Map View");
                        MapView::show(ui, &mut self.map_tiles, &mut self.map_memory, &self.telem_snapshot);
                    });
                });
                
                ui.separator();
                
                ui.allocate_ui(egui::Vec2::new(ui.available_width(), ui.available_height()), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Video Receiver");
                        VideoView::show(ui, &mut self.video_texture);
                    });
                });
            });
        });
    }
}
