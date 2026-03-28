use anyhow::{anyhow, Result};
use gstreamer::prelude::*;
use gstreamer::Pipeline;
use gstreamer_app::AppSink;
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct VideoReceiver {
    pipeline: Pipeline,
    latest_frame: Arc<Mutex<Option<VideoFrame>>>,
}

impl VideoReceiver {
    /// Create a new video receiver listening on a specific UDP port
    pub fn new(port: u16) -> Result<Self> {
        gstreamer::init()?;

        // Example pipeline for an incoming H.264 UDP stream:
        // udpsrc port=5600 ! application/x-rtp, payload=96 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! video/x-raw,format=RGBA ! appsink
        let pipeline_str = format!(
            "udpsrc port={} ! application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, payload=96 \
             ! rtph264depay ! h264parse ! queue ! avdec_h264 ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink drop=true max-buffers=1",
            port
        );

        let pipeline = gstreamer::parse::launch(&pipeline_str)?
            .downcast::<Pipeline>()
            .map_err(|_| anyhow!("Failed to create pipeline"))?;

        let sink: AppSink = pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow!("Failed to find appsink in pipeline"))?
            .downcast()
            .map_err(|_| anyhow!("Sink element is not an appsink"))?;

        let latest_frame = Arc::new(Mutex::new(None));
        let frame_writer = latest_frame.clone();

        // Callback when a new frame arrives
        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                    
                    let caps = sample.caps().ok_or(gstreamer::FlowError::Error)?;
                    let structure = caps.structure(0).ok_or(gstreamer::FlowError::Error)?;
                    
                    let width = structure.get::<i32>("width").map_err(|_| gstreamer::FlowError::Error)? as u32;
                    let height = structure.get::<i32>("height").map_err(|_| gstreamer::FlowError::Error)? as u32;

                    let map = buffer.map_readable().map_err(|_| gstreamer::FlowError::Error)?;
                    
                    // Create a copy of the pixel data to send to UI
                    let frame = VideoFrame {
                        width,
                        height,
                        data: map.as_slice().to_vec(),
                    };

                    *frame_writer.lock().unwrap() = Some(frame);
                    
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );

        Ok(Self {
            pipeline,
            latest_frame,
        })
    }

    /// Start playing the pipeline
    pub fn start(&self) -> Result<()> {
        info!("Starting video receiver pipeline");
        self.pipeline.set_state(gstreamer::State::Playing)?;
        Ok(())
    }

    /// Stop the pipeline
    pub fn stop(&self) -> Result<()> {
        info!("Stopping video receiver pipeline");
        self.pipeline.set_state(gstreamer::State::Null)?;
        Ok(())
    }

    /// Get the latest decorded frame (clears it after reading)
    pub fn take_latest_frame(&self) -> Option<VideoFrame> {
        self.latest_frame.lock().unwrap().take()
    }
    
    /// True if pipeline is playing
    pub fn is_playing(&self) -> bool {
        let (_, state, _) = self.pipeline.state(gstreamer::ClockTime::NONE);
        state == gstreamer::State::Playing
    }
}

impl Drop for VideoReceiver {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
