use eframe::egui;
use uav_protocol::TelemetryData;
use walkers::{Map, MapMemory, Plugin, Projector, Tiles, lon_lat};

pub struct MapView;

struct DroneMarker<'a> {
    telem: &'a TelemetryData,
}

impl<'a> Plugin for DroneMarker<'a> {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        _response: &egui::Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        if self.telem.position.lat == 0.0 && self.telem.position.lon == 0.0 {
            return; // No valid GPS
        }

        let pos = lon_lat(self.telem.position.lon, self.telem.position.lat);
        let screen_pos = projector.project(pos).to_pos2();
        
        // Yaw in radians (forward direction)
        let yaw = self.telem.attitude.yaw;
        
        // Basic drone drawing (red circle with pointing line)
        // Note: 0 yaw is North (up), so X = sin(yaw), Y = -cos(yaw)
        let dir = egui::Vec2::new(yaw.sin(), -yaw.cos());
        
        ui.painter().line_segment(
            [screen_pos, screen_pos + dir * 25.0],
            egui::Stroke::new(3.0, egui::Color32::RED),
        );
        ui.painter().circle_filled(screen_pos, 8.0, egui::Color32::RED);
        ui.painter().circle_stroke(
            screen_pos,
            8.0,
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
    }
}

impl MapView {
    pub fn show<T: Tiles>(
        ui: &mut egui::Ui,
        tiles: &mut T,
        map_memory: &mut MapMemory,
        telem: &TelemetryData,
    ) {
        let has_gps = telem.position.lat != 0.0 || telem.position.lon != 0.0;
        
        let pos = if has_gps {
            lon_lat(telem.position.lon, telem.position.lat)
        } else {
            // Default center if no GPS data
            lon_lat(105.8048, 21.0285)
        };
        
        let map = Map::new(Some(tiles), map_memory, pos)
            .with_plugin(DroneMarker { telem });
            
        ui.add(map);
    }
}
