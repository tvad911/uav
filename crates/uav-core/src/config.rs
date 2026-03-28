use serde::{Deserialize, Serialize};
use uav_protocol::ConnectionType;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Connection settings
    pub connection: ConnectionType,
    /// Telemetry update rates
    pub telemetry: TelemetryConfig,
    /// Gamepad settings
    pub gamepad: GamepadConfig,
    /// UI settings
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Data stream request rate in Hz
    pub stream_rate_hz: u16,
    /// Heartbeat timeout in milliseconds
    pub heartbeat_timeout_ms: u64,
    /// Heartbeat send interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadConfig {
    /// Enable gamepad control
    pub enabled: bool,
    /// Deadzone for stick input (0.0 - 1.0)
    pub deadzone: f32,
    /// Expo curve factor (1.0 = linear)
    pub expo: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Window width
    pub width: u32,
    /// Window height
    pub height: u32,
    /// Dark mode
    pub dark_mode: bool,
    /// Map tile server URL
    pub map_tile_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionType::default(),
            telemetry: TelemetryConfig {
                stream_rate_hz: 10,
                heartbeat_timeout_ms: 3000,
                heartbeat_interval_ms: 1000,
            },
            gamepad: GamepadConfig {
                enabled: true,
                deadzone: 0.05,
                expo: 1.5,
            },
            ui: UiConfig {
                width: 1280,
                height: 800,
                dark_mode: true,
                map_tile_url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
            },
        }
    }
}

impl AppConfig {
    /// Load config from file, or create default if not found
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("uav-gcs");

        let config_path = config_dir.join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("uav-gcs");

        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
