use gilrs::{Axis, Event, EventType, Gilrs};
use tracing::{info, warn};

use crate::config::GamepadConfig;

/// Gamepad axis values normalized to -1.0 to 1.0
#[derive(Debug, Clone, Copy, Default)]
pub struct GamepadState {
    /// Aileron (Roll) - Left stick X
    pub roll: f32,
    /// Elevator (Pitch) - Left stick Y
    pub pitch: f32,
    /// Throttle - Right stick Y
    pub throttle: f32,
    /// Rudder (Yaw) - Right stick X
    pub yaw: f32,
    /// Whether any gamepad is connected
    pub connected: bool,
}

impl GamepadState {
    /// Convert gamepad state to RC PWM channels (1000-2000)
    pub fn to_rc_channels(&self) -> [u16; 8] {
        [
            axis_to_pwm(self.roll),     // CH1 Aileron
            axis_to_pwm(self.pitch),    // CH2 Elevator
            axis_to_pwm(self.throttle), // CH3 Throttle
            axis_to_pwm(self.yaw),      // CH4 Rudder
            1500, // CH5 - not mapped
            1500, // CH6 - not mapped
            1500, // CH7 - not mapped
            1500, // CH8 - not mapped
        ]
    }
}

fn axis_to_pwm(value: f32) -> u16 {
    // Map -1.0..1.0 to 1000..2000
    let pwm = 1500.0 + (value * 500.0);
    pwm.clamp(1000.0, 2000.0) as u16
}

/// Apply deadzone and expo curve to axis value
fn apply_deadzone_and_expo(raw: f32, deadzone: f32, expo: f32) -> f32 {
    let abs_val = raw.abs();
    if abs_val < deadzone {
        return 0.0;
    }

    // Rescale from deadzone..1.0 to 0.0..1.0
    let normalized = (abs_val - deadzone) / (1.0 - deadzone);

    // Apply expo curve: output = input^expo
    let expo_val = normalized.powf(expo);

    expo_val.copysign(raw)
}

/// Manages gamepad input for RC override control
pub struct GamepadManager {
    gilrs: Option<Gilrs>,
    config: GamepadConfig,
    state: GamepadState,
}

impl GamepadManager {
    pub fn new(config: GamepadConfig) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_id, gamepad) in g.gamepads() {
                    info!("Gamepad detected: {} ({:?})", gamepad.name(), gamepad.power_info());
                }
                Some(g)
            }
            Err(e) => {
                warn!("Failed to initialize gamepad system: {}", e);
                None
            }
        };

        Self {
            gilrs,
            config,
            state: GamepadState::default(),
        }
    }

    /// Poll gamepad events and update state.
    /// Returns the current gamepad state.
    pub fn poll(&mut self) -> GamepadState {
        let Some(gilrs) = &mut self.gilrs else {
            return self.state;
        };

        let deadzone = self.config.deadzone;
        let expo = self.config.expo;

        // Process all pending events
        while let Some(Event { id: _, event, .. }) = gilrs.next_event() {
            match event {
                EventType::AxisChanged(axis, value, _) => {
                    let value = apply_deadzone_and_expo(value, deadzone, expo);
                    match axis {
                        Axis::LeftStickX => self.state.roll = value,
                        Axis::LeftStickY => self.state.pitch = -value, // Invert Y for pitch
                        Axis::RightStickX => self.state.yaw = value,
                        Axis::RightStickY => self.state.throttle = value,
                        _ => {}
                    }
                }
                EventType::Connected => {
                    info!("Gamepad connected");
                    self.state.connected = true;
                }
                EventType::Disconnected => {
                    warn!("Gamepad disconnected");
                    self.state.connected = false;
                    self.state = GamepadState::default();
                }
                _ => {}
            }
        }

        self.state
    }

    pub fn state(&self) -> &GamepadState {
        &self.state
    }
}
