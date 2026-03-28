use eframe::egui;
use uav_protocol::DetectionFrame;

pub struct DetectionOverlay;

impl DetectionOverlay {
    /// Draw AI detection bounding boxes on top of the video feed area.
    /// `video_rect` is the screen rect where the video is displayed.
    pub fn draw(ui: &mut egui::Ui, video_rect: egui::Rect, detections: &DetectionFrame) {
        if detections.detections.is_empty() {
            return;
        }

        let painter = ui.painter();

        for det in &detections.detections {
            // Convert normalized bbox to screen coordinates
            let x = video_rect.min.x + det.bbox.x * video_rect.width();
            let y = video_rect.min.y + det.bbox.y * video_rect.height();
            let w = det.bbox.w * video_rect.width();
            let h = det.bbox.h * video_rect.height();

            let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

            // Color based on object class
            let color = class_color(&det.label);

            // Draw bounding box
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(2),
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Outside,
            );

            // Draw label background + text
            let label_text = if let Some(dist) = det.distance_m {
                format!("{} {:.0}% ({:.1}m)", det.label, det.confidence * 100.0, dist)
            } else {
                format!("{} {:.0}%", det.label, det.confidence * 100.0)
            };

            let label_pos = egui::pos2(x, y - 16.0);
            let text_galley = painter.layout_no_wrap(
                label_text.clone(),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            let text_rect = egui::Rect::from_min_size(
                label_pos,
                text_galley.size() + egui::vec2(6.0, 2.0),
            );

            painter.rect_filled(text_rect, egui::CornerRadius::same(2), color.gamma_multiply(0.8));
            painter.galley(label_pos + egui::vec2(3.0, 1.0), text_galley, color);
        }

        // Detection counter at top-right
        let count_text = format!("🎯 {} objects", detections.detections.len());
        let counter_pos = egui::pos2(
            video_rect.max.x - 120.0,
            video_rect.min.y + 5.0,
        );
        painter.text(
            counter_pos,
            egui::Align2::LEFT_TOP,
            count_text,
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(255, 200, 50),
        );
    }
}

/// Pick a color for each detection class
fn class_color(label: &str) -> egui::Color32 {
    match label.to_lowercase().as_str() {
        "person" => egui::Color32::from_rgb(255, 100, 100),    // Red
        "car" => egui::Color32::from_rgb(100, 200, 255),       // Blue
        "truck" => egui::Color32::from_rgb(100, 255, 200),     // Cyan
        "bicycle" => egui::Color32::from_rgb(255, 200, 100),   // Orange
        "building" => egui::Color32::from_rgb(200, 150, 255),  // Purple
        "animal" => egui::Color32::from_rgb(150, 255, 100),    // Green
        _ => egui::Color32::from_rgb(255, 255, 100),           // Yellow (default)
    }
}
