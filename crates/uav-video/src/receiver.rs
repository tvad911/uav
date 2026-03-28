use thiserror::Error;
use tracing::info;

/// Video receiver configuration
#[derive(Debug, Clone)]
pub struct VideoConfig {
    /// Source type: "udp", "rtsp", "v4l2" (USB capture card)
    pub source: VideoSource,
    /// Target resolution width
    pub width: u32,
    /// Target resolution height
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum VideoSource {
    /// UDP stream (e.g., from analog VRX with USB adapter)
    Udp { address: String, port: u16 },
    /// RTSP stream (e.g., IP camera)
    Rtsp { url: String },
    /// V4L2 device (USB capture card for analog video)
    V4L2 { device: String },
    /// Test pattern (for development without hardware)
    TestPattern,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            source: VideoSource::TestPattern,
            width: 1280,
            height: 720,
        }
    }
}

/// Video frame data (raw RGBA pixels)
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA
    pub timestamp_ms: u64,
}

#[derive(Debug, Error)]
pub enum VideoError {
    #[error("Failed to open video source: {0}")]
    OpenFailed(String),
    #[error("Decode error: {0}")]
    DecodeFailed(String),
    #[error("Connection lost")]
    ConnectionLost,
}

/// Video receiver - handles video stream reception and decoding.
///
/// Current implementation provides a test pattern.
/// Will be expanded with GStreamer/FFmpeg backend when hardware is available.
pub struct VideoReceiver {
    config: VideoConfig,
    running: bool,
    frame_count: u64,
}

impl VideoReceiver {
    pub fn new(config: VideoConfig) -> Self {
        Self {
            config,
            running: false,
            frame_count: 0,
        }
    }

    /// Start receiving video
    pub fn start(&mut self) -> Result<(), VideoError> {
        info!("Starting video receiver: {:?}", self.config.source);
        self.running = true;
        Ok(())
    }

    /// Stop receiving video
    pub fn stop(&mut self) {
        self.running = false;
        info!("Video receiver stopped");
    }

    /// Get the next video frame.
    /// Returns a test pattern for development.
    pub fn next_frame(&mut self) -> Option<VideoFrame> {
        if !self.running {
            return None;
        }

        self.frame_count += 1;

        match &self.config.source {
            VideoSource::TestPattern => Some(self.generate_test_pattern()),
            _ => {
                // TODO: Implement real video capture
                // For now, return test pattern for all sources
                Some(self.generate_test_pattern())
            }
        }
    }

    /// Generate a test pattern frame for development/testing
    fn generate_test_pattern(&self) -> VideoFrame {
        let w = self.config.width;
        let h = self.config.height;
        let mut data = vec![0u8; (w * h * 4) as usize];

        let t = self.frame_count as f32 * 0.02;

        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                let fx = x as f32 / w as f32;
                let fy = y as f32 / h as f32;

                // Animated gradient pattern
                let r = ((fx * 255.0 + t * 50.0) % 255.0) as u8;
                let g = ((fy * 255.0 + t * 30.0) % 255.0) as u8;
                let b = (((fx + fy) * 127.0 + t * 20.0) % 255.0) as u8;

                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255; // Alpha
            }
        }

        // Draw crosshair in center
        let cx = w / 2;
        let cy = h / 2;
        let crosshair_size: u32 = 40;

        for i in 0..crosshair_size {
            // Horizontal line
            if cx + i < w {
                let idx = ((cy * w + cx + i) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
            }
            if cx >= i {
                let idx = ((cy * w + cx - i) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
            }
            // Vertical line
            if cy + i < h {
                let idx = (((cy + i) * w + cx) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
            }
            if cy >= i {
                let idx = (((cy - i) * w + cx) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
            }
        }

        VideoFrame {
            width: w,
            height: h,
            data,
            timestamp_ms: self.frame_count * 33, // ~30fps
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}
