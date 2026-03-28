use eframe::egui;
use uav_protocol::TelemetryData;

/// Left panel showing telemetry gauges and values
pub struct TelemetryPanel;

impl TelemetryPanel {
    pub fn show(ui: &mut egui::Ui, telem: &TelemetryData, data_rate_hz: f32) {
        ui.heading("Telemetry");
        ui.separator();

        // Let the panel scroll if content overflows
        egui::ScrollArea::vertical().show(ui, |ui| {
            // -- Flight stats --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Session & Flight Time").strong());
                
                let mins = (telem.armed_time_s / 60.0).floor() as u32;
                let secs = (telem.armed_time_s % 60.0) as u32;
                ui.label(
                    egui::RichText::new(format!("Flight Time: {:02}:{:02}", mins, secs))
                        .color(egui::Color32::from_rgb(100, 255, 100))
                        .size(16.0)
                        .strong(),
                );
            });
            ui.add_space(4.0);

            // -- Speed --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Speed").strong());
                egui::Grid::new("speed_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Airspeed:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} m/s", telem.airspeed))
                                .color(egui::Color32::from_rgb(100, 200, 255))
                                .strong(),
                        );
                        ui.end_row();

                        ui.label("Groundspeed:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} m/s", telem.ground_speed))
                                .color(egui::Color32::from_rgb(100, 200, 255)),
                        );
                        ui.end_row();

                        ui.label("Climb:");
                        let climb_color = if telem.climb_rate > 0.5 {
                            egui::Color32::GREEN
                        } else if telem.climb_rate < -0.5 {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.label(
                            egui::RichText::new(format!("{:.1} m/s", telem.climb_rate))
                                .color(climb_color),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(4.0);

            // -- Altitude --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Altitude").strong());
                egui::Grid::new("alt_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("MSL:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} m", telem.position.alt_msl))
                                .color(egui::Color32::from_rgb(255, 200, 100)),
                        );
                        ui.end_row();

                        ui.label("AGL:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} m", telem.position.alt_agl))
                                .color(egui::Color32::from_rgb(255, 200, 100))
                                .strong(),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(4.0);

            // -- Attitude (Text) --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Attitude & Heading").strong());
                egui::Grid::new("att_grid").num_columns(2).striped(true).show(ui, |ui| {
                    ui.label("Heading:");
                    ui.label(format!("{}°", telem.heading));
                    ui.end_row();

                    ui.label("Pitch:");
                    ui.label(format!("{:.1}°", telem.attitude.pitch));
                    ui.end_row();

                    ui.label("Roll:");
                    ui.label(format!("{:.1}°", telem.attitude.roll));
                    ui.end_row();
                });
            });

            ui.add_space(4.0);

            // -- Battery --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Battery").strong());

                // Battery bar
                let pct = telem.battery.remaining_pct.max(0) as f32 / 100.0;
                let bar_color = if pct > 0.5 {
                    egui::Color32::GREEN
                } else if pct > 0.2 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                let bar = egui::ProgressBar::new(pct)
                    .text(format!("{}%", telem.battery.remaining_pct))
                    .fill(bar_color);
                ui.add(bar);

                egui::Grid::new("batt_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Voltage:");
                        ui.label(
                            egui::RichText::new(format!("{:.2} V", telem.battery.voltage))
                                .color(egui::Color32::from_rgb(200, 200, 100)),
                        );
                        ui.end_row();

                        ui.label("Current:");
                        ui.label(format!("{:.1} A", telem.battery.current));
                        ui.end_row();

                        ui.label("Used:");
                        ui.label(format!("{} mAh", telem.battery.mah_consumed));
                        ui.end_row();
                    });
            });

            ui.add_space(4.0);

            // -- GPS & Navigation --
            ui.group(|ui| {
                ui.label(egui::RichText::new("GPS & Nav").strong());
                egui::Grid::new("gps_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Fix:");
                        let fix_text = format!("{:?}", telem.gps_fix);
                        let fix_color = match telem.gps_fix {
                            uav_protocol::GpsFixType::Fix3D
                            | uav_protocol::GpsFixType::DGps
                            | uav_protocol::GpsFixType::RtkFixed => egui::Color32::GREEN,
                            uav_protocol::GpsFixType::Fix2D
                            | uav_protocol::GpsFixType::RtkFloat => egui::Color32::YELLOW,
                            _ => egui::Color32::RED,
                        };
                        ui.label(egui::RichText::new(fix_text).color(fix_color));
                        ui.end_row();

                        ui.label("Sats | HDOP:");
                        ui.label(format!("{} | {:.1}", telem.gps_satellites, telem.gps_hdop));
                        ui.end_row();

                        ui.label("Home dist:");
                        ui.label(format!("{:.0} m", telem.distance_to_home));
                        ui.end_row();

                        ui.label("Home brng:");
                        ui.label(format!("{:.0}°", telem.bearing_to_home));
                        ui.end_row();

                        if telem.current_wp > 0 {
                            ui.label("Target WP:");
                            ui.label(format!("#{} ({:.0}m)", telem.current_wp, telem.distance_to_wp));
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(4.0);

            // -- Data Link --
            ui.group(|ui| {
                ui.label(egui::RichText::new("Data Link & RC").strong());
                
                let rssi_pct = telem.rssi as f32 / 255.0;
                let rssi_color = if rssi_pct > 0.5 {
                    egui::Color32::GREEN
                } else if rssi_pct > 0.2 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.add(
                    egui::ProgressBar::new(rssi_pct)
                        .text(format!("RC RSSI: {}", telem.rssi))
                        .fill(rssi_color),
                );
                
                ui.add_space(4.0);
                egui::Grid::new("link_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Data Rate:");
                        ui.label(format!("{:.1} Hz", data_rate_hz));
                        ui.end_row();
                        
                        ui.label("Total Msgs:");
                        ui.label(format!("{}", telem.messages_received));
                        ui.end_row();
                        
                        ui.label("Heartbeat:");
                        ui.label(if telem.heartbeat_ok {
                            egui::RichText::new("OK").color(egui::Color32::GREEN)
                        } else {
                            egui::RichText::new("LOST").color(egui::Color32::RED).strong()
                        });
                        ui.end_row();
                    });
            });
        });
    }
}
