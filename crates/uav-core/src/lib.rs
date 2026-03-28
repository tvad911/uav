// UAV Core - Communication, telemetry processing, and mission management

pub mod config;
pub mod connection;
pub mod gamepad;
pub mod heartbeat;
pub mod telemetry_handler;

pub use config::AppConfig;
pub use connection::MavConnection;
pub use gamepad::GamepadManager;
pub use heartbeat::HeartbeatManager;
pub use telemetry_handler::TelemetryHandler;
