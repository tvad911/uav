use eframe::egui;
use uav_protocol::TelemetryData;

/// Artificial horizon / attitude indicator widget
pub struct AttitudeIndicator;

impl AttitudeIndicator {
    pub fn show(ui: &mut egui::Ui, telem: &TelemetryData) {
        let available = ui.available_size();
        let size = available.x.min(available.y).min(300.0);
        let (response, painter) =
            ui.allocate_painter(egui::Vec2::new(size, size), egui::Sense::hover());
        let center = response.rect.center();
        let radius = size / 2.0 - 10.0;

        let roll_rad = telem.attitude.roll.to_radians();
        let pitch_px = telem.attitude.pitch * (radius / 45.0); // 45° = full scale

        // Background circle (clip region)
        painter.circle_filled(center, radius, egui::Color32::from_rgb(20, 60, 120));

        // Sky (upper half, shifted by pitch)
        let sky_rect = egui::Rect::from_center_size(
            center + egui::Vec2::new(0.0, -pitch_px),
            egui::Vec2::new(size * 2.0, size * 2.0),
        );
        painter.rect_filled(
            egui::Rect::from_min_max(
                sky_rect.min,
                egui::Pos2::new(sky_rect.max.x, center.y - pitch_px),
            ),
            0.0,
            egui::Color32::from_rgb(50, 120, 200),
        );

        // Ground (lower half, shifted by pitch)
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::Pos2::new(sky_rect.min.x, center.y - pitch_px),
                sky_rect.max,
            ),
            0.0,
            egui::Color32::from_rgb(100, 70, 30),
        );

        // Horizon line
        let _horizon_y = center.y - pitch_px;
        let h_left = center + egui::Vec2::new(-radius, -pitch_px);
        let h_right = center + egui::Vec2::new(radius, -pitch_px);
        // Apply roll rotation
        let rotated_left = rotate_point(h_left, center, roll_rad);
        let rotated_right = rotate_point(h_right, center, roll_rad);
        painter.line_segment(
            [rotated_left, rotated_right],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );

        // Pitch ladder lines (every 10°)
        for pitch_line in [-30_i32, -20, -10, 10, 20, 30] {
            let py = center.y - pitch_px - (pitch_line as f32 * radius / 45.0);
            let half_w = if pitch_line.abs() == 10 { 30.0 } else { 20.0 };
            let left = rotate_point(
                egui::Pos2::new(center.x - half_w, py),
                center,
                roll_rad,
            );
            let right = rotate_point(
                egui::Pos2::new(center.x + half_w, py),
                center,
                roll_rad,
            );
            painter.line_segment(
                [left, right],
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(150)),
            );
        }

        // Center aircraft symbol (fixed)
        let wing_len = 40.0;
        painter.line_segment(
            [
                center + egui::Vec2::new(-wing_len, 0.0),
                center + egui::Vec2::new(-10.0, 0.0),
            ],
            egui::Stroke::new(3.0, egui::Color32::YELLOW),
        );
        painter.line_segment(
            [
                center + egui::Vec2::new(10.0, 0.0),
                center + egui::Vec2::new(wing_len, 0.0),
            ],
            egui::Stroke::new(3.0, egui::Color32::YELLOW),
        );
        painter.circle_filled(center, 4.0, egui::Color32::YELLOW);

        // Circle border
        painter.circle_stroke(
            center,
            radius,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80)),
        );

        // Roll/pitch text overlay
        let text_pos = response.rect.left_bottom() + egui::Vec2::new(5.0, -20.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_BOTTOM,
            format!(
                "Roll: {:.1}° | Pitch: {:.1}° | Hdg: {}°",
                telem.attitude.roll, telem.attitude.pitch, telem.heading
            ),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
}

/// Rotate a point around a center by angle (radians)
fn rotate_point(point: egui::Pos2, center: egui::Pos2, angle: f32) -> egui::Pos2 {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    egui::Pos2::new(
        center.x + dx * cos_a - dy * sin_a,
        center.y + dx * sin_a + dy * cos_a,
    )
}
