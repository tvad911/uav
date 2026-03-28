use eframe::egui;
use uav_protocol::CompanionStatus;

pub struct CompanionPanel;

impl CompanionPanel {
    pub fn show(ui: &mut egui::Ui, status: &CompanionStatus) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // -- Connection Status --
            ui.group(|ui| {
                ui.heading("📱 Companion Computer");
                ui.separator();

                let (status_text, color) = if status.connected {
                    ("● CONNECTED", egui::Color32::from_rgb(80, 200, 80))
                } else {
                    ("○ DISCONNECTED", egui::Color32::from_rgb(200, 80, 80))
                };
                ui.label(egui::RichText::new(status_text).strong().color(color));

                if status.connected {
                    let uptime_min = status.uptime_s / 60;
                    let uptime_sec = status.uptime_s % 60;
                    ui.label(format!("Uptime: {:02}:{:02}", uptime_min, uptime_sec));
                }
            });

            ui.add_space(4.0);

            // -- 4G Signal --
            ui.group(|ui| {
                ui.label(egui::RichText::new("📶 4G Data Link").strong());
                ui.separator();

                let signal_quality = match status.signal_dbm {
                    -70..=0 => ("Excellent", egui::Color32::from_rgb(80, 200, 80)),
                    -85..=-71 => ("Good", egui::Color32::from_rgb(150, 200, 80)),
                    -100..=-86 => ("Fair", egui::Color32::YELLOW),
                    _ => ("Poor", egui::Color32::RED),
                };

                ui.horizontal(|ui| {
                    ui.label("Signal:");
                    ui.label(
                        egui::RichText::new(format!("{} ({}dBm)", signal_quality.0, status.signal_dbm))
                            .color(signal_quality.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Network:");
                    ui.label(&status.network_type);
                });
                ui.horizontal(|ui| {
                    ui.label(format!("↑ {:.1} KB/s", status.upload_kbps));
                    ui.label(format!("↓ {:.1} KB/s", status.download_kbps));
                });
            });

            ui.add_space(4.0);

            // -- System Resources --
            ui.group(|ui| {
                ui.label(egui::RichText::new("💻 System").strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("CPU:");
                    let cpu_color = if status.cpu_usage > 80.0 {
                        egui::Color32::RED
                    } else if status.cpu_usage > 50.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::from_rgb(80, 200, 80)
                    };
                    ui.label(
                        egui::RichText::new(format!("{:.0}%", status.cpu_usage)).color(cpu_color),
                    );
                });

                // CPU temp
                let temp_color = if status.cpu_temp > 70.0 {
                    egui::Color32::RED
                } else if status.cpu_temp > 50.0 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::from_rgb(80, 200, 80)
                };
                ui.horizontal(|ui| {
                    ui.label("Temp:");
                    ui.label(
                        egui::RichText::new(format!("{:.0}°C", status.cpu_temp)).color(temp_color),
                    );
                });

                // RAM
                let ram_pct = if status.ram_total_mb > 0 {
                    status.ram_used_mb as f32 / status.ram_total_mb as f32
                } else {
                    0.0
                };
                ui.horizontal(|ui| {
                    ui.label("RAM:");
                    ui.label(format!(
                        "{}/{} MB ({:.0}%)",
                        status.ram_used_mb,
                        status.ram_total_mb,
                        ram_pct * 100.0
                    ));
                });

                // Battery
                if status.battery_pct >= 0 {
                    let batt_color = if status.battery_pct < 20 {
                        egui::Color32::RED
                    } else if status.battery_pct < 50 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::from_rgb(80, 200, 80)
                    };
                    ui.horizontal(|ui| {
                        ui.label("Battery:");
                        ui.label(
                            egui::RichText::new(format!("{}%", status.battery_pct))
                                .color(batt_color),
                        );
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Power:");
                        ui.label(
                            egui::RichText::new("External (BEC)")
                                .color(egui::Color32::from_rgb(80, 200, 80)),
                        );
                    });
                }
            });

            ui.add_space(4.0);

            // -- AI Status --
            ui.group(|ui| {
                ui.label(egui::RichText::new("🧠 AI Engine").strong());
                ui.separator();

                let (ai_text, ai_color) = if status.ai_running {
                    (
                        format!("RUNNING ({:.1} FPS)", status.ai_fps),
                        egui::Color32::from_rgb(80, 200, 80),
                    )
                } else {
                    ("STOPPED".to_string(), egui::Color32::GRAY)
                };
                ui.label(egui::RichText::new(ai_text).color(ai_color));

                let stream_text = if status.streaming {
                    "● Streaming"
                } else {
                    "○ No Stream"
                };
                let stream_color = if status.streaming {
                    egui::Color32::from_rgb(80, 200, 80)
                } else {
                    egui::Color32::GRAY
                };
                ui.label(egui::RichText::new(stream_text).color(stream_color));
            });
        });
    }
}
