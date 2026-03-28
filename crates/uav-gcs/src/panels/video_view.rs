use eframe::egui;

// We need a stable texture Handle across frames
// We can store that inside GcsApp or manage it statically if there's only one.
// The best is keeping it inside VideoReceiver or GcsApp. We will pass a mutable texture.

pub struct VideoView;

impl VideoView {
    pub fn show(ui: &mut egui::Ui, texture: &mut Option<egui::TextureHandle>) {
        if let Some(tex) = texture {
            let available_size = ui.available_size();
            let tex_size = tex.size_vec2();
            
            // Calculate scale to fit aspect ratio
            let scale_x = available_size.x / tex_size.x;
            let scale_y = available_size.y / tex_size.y;
            let scale = scale_x.min(scale_y);
            
            let display_size = tex_size * scale;
            
            ui.vertical_centered(|ui| {
                ui.image((tex.id(), display_size));
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Waiting for video stream on UDP port 5600...")
                        .color(egui::Color32::from_rgb(150, 150, 150))
                        .italics(),
                );
            });
        }
    }
}
