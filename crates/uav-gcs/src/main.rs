use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use uav_core::{AppConfig, MavConnection};
use uav_protocol::{BackendEvent, TelemetryData, UiCommand};

mod app;
pub mod panels;

use app::GcsApp;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("uav_gcs=info".parse().unwrap())
                .add_directive("uav_core=info".parse().unwrap())
                .add_directive("uav_protocol=info".parse().unwrap()),
        )
        .init();

    info!("Starting UAV Ground Control Station");

    // Load configuration
    let config = AppConfig::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    // Shared telemetry state (read by UI, written by background task)
    let telemetry = Arc::new(RwLock::new(TelemetryData::new()));

    // Channels for UI ↔ Backend communication
    let (ui_cmd_tx, mut ui_cmd_rx) = mpsc::channel::<UiCommand>(64);
    let (backend_evt_tx, backend_evt_rx) = mpsc::channel::<BackendEvent>(64);

    // Create tokio runtime for background MAVLink tasks
    let rt = Runtime::new().expect("Failed to create tokio runtime");

    // Spawn background connection manager task
    let telemetry_bg = telemetry.clone();
    let backend_tx = backend_evt_tx.clone();
    rt.spawn(async move {
        let mut connection: Option<MavConnection> = None;
        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        loop {
            match ui_cmd_rx.recv().await {
                Some(UiCommand::Connect(conn_type)) => {
                    info!("UI requested connection: {:?}", conn_type);

                    // Clean up previous connection
                    if let Some(conn) = connection.take() {
                        conn.disconnect().await;
                        for h in handles.drain(..) {
                            h.abort();
                        }
                    }

                    let conn = MavConnection::new(conn_type, telemetry_bg.clone());

                    match conn.connect().await {
                        Ok(new_handles) => {
                            handles = new_handles;
                            let _ = backend_tx.send(BackendEvent::Connected).await;
                            connection = Some(conn);
                        }
                        Err(e) => {
                            let _ = backend_tx
                                .send(BackendEvent::Error(format!("Connection failed: {}", e)))
                                .await;
                        }
                    }
                }
                Some(UiCommand::Disconnect) => {
                    info!("UI requested disconnect");
                    if let Some(conn) = connection.take() {
                        conn.disconnect().await;
                        for h in handles.drain(..) {
                            h.abort();
                        }
                        let _ = backend_tx
                            .send(BackendEvent::Disconnected("User disconnected".to_string()))
                            .await;
                    }
                }
                Some(UiCommand::SendCommand(cmd)) => {
                    if let Some(conn) = &connection {
                        let sender = conn.command_sender();
                        if let Err(e) = sender.send(cmd).await {
                            let _ = backend_tx
                                .send(BackendEvent::Error(format!("Failed to send command: {}", e)))
                                .await;
                        }
                    } else {
                        let _ = backend_tx
                            .send(BackendEvent::Error("Not connected".to_string()))
                            .await;
                    }
                }
                None => {
                    info!("UI command channel closed, shutting down backend");
                    break;
                }
            }
        }

        // Clean up on exit
        if let Some(conn) = connection.take() {
            conn.disconnect().await;
        }
        for h in handles {
            h.abort();
        }
    });

    // Setup egui window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.ui.width as f32, config.ui.height as f32])
            .with_min_inner_size([800.0, 600.0])
            .with_title("UAV Ground Control Station"),
        ..Default::default()
    };

    let telemetry_ui = telemetry.clone();
    let config_clone = config.clone();

    eframe::run_native(
        "UAV GCS",
        options,
        Box::new(move |cc| {
            Ok(Box::new(GcsApp::new(
                cc,
                telemetry_ui,
                config_clone,
                ui_cmd_tx,
                backend_evt_rx,
            )))
        }),
    )
}
