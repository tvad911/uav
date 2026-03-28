use eframe::egui;
use uav_protocol::{ArmState, TelemetryData};

/// Bottom status bar showing critical info at a glance
pub struct StatusBar;

impl StatusBar {
    pub fn show(ctx: &egui::Context, telem: &TelemetryData, connected: bool, data_rate_hz: f32) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Connection status
                let (conn_icon, _conn_color) = if connected {
                    if telem.heartbeat_ok {
                        ("🟢", egui::Color32::GREEN)
                    } else {
                        ("🟡", egui::Color32::YELLOW)
                    }
                } else {
                    ("🔴", egui::Color32::RED)
                };
                ui.label(egui::RichText::new(conn_icon));

                ui.separator();

                // Flight mode
                ui.label(
                    egui::RichText::new(telem.flight_mode.name())
                        .strong()
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );

                ui.separator();

                // Arm state
                match telem.arm_state {
                    ArmState::Armed => {
                        let mins = (telem.armed_time_s / 60.0).floor() as u32;
                        let secs = (telem.armed_time_s % 60.0) as u32;
                        ui.label(
                            egui::RichText::new(format!("ARMED [{:02}:{:02}]", mins, secs))
                                .strong()
                                .color(egui::Color32::RED),
                        );
                    }
                    ArmState::Disarmed => {
                        ui.label(
                            egui::RichText::new("DISARMED")
                                .color(egui::Color32::GREEN),
                        );
                    }
                }

                ui.separator();

                // Battery
                let batt_color = if telem.battery.remaining_pct > 50 {
                    egui::Color32::GREEN
                } else if telem.battery.remaining_pct > 20 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.label(
                    egui::RichText::new(format!(
                        "🔋 {:.1}V {}%",
                        telem.battery.voltage, telem.battery.remaining_pct
                    ))
                    .color(batt_color),
                );

                ui.separator();

                // GPS
                ui.label(format!("🛰 {} sats", telem.gps_satellites));

                ui.separator();

                // Speed & altitude
                ui.label(format!(
                    "AS:{:.0} GS:{:.0} m/s | Alt:{:.0}m",
                    telem.airspeed, telem.ground_speed, telem.position.alt_agl
                ));

                ui.separator();

                // Home distance
                ui.label(format!("🏠 {:.0}m", telem.distance_to_home));

                ui.separator();
                
                // Data Rate
                ui.label(format!("📶 {:.1} Hz", data_rate_hz));

                // Status text (right-aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(text) = &telem.status_text {
                        ui.label(
                            egui::RichText::new(text)
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    }
                });
            });
        });
    }
}
