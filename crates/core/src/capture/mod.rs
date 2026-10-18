// crates/core/src/capture/mod.rs
// Cross-platform screen capture abstraction using xcap (Windows, macOS, Linux)

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use xcap::Monitor;

/// Raw frame from screen capture
pub struct Frame {
    /// Raw pixel data
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Screen capture trait — platform agnostic interface
pub trait ScreenCapture: Send {
    fn capture(&mut self) -> Result<Frame>;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
}

pub struct PlatformCapture {
    monitor: Monitor,
    width: u32,
    height: u32,
}

impl PlatformCapture {
    pub fn new() -> Result<Self> {
        let monitors = Monitor::all().map_err(|e| anyhow::anyhow!("Failed to list monitors: {:?}", e))?;
        let monitor = monitors
            .into_iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .or_else(|| Monitor::all().ok()?.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let width = monitor.width().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let height = monitor.height().map_err(|e| anyhow::anyhow!("{:?}", e))?;

        Ok(Self {
            monitor,
            width,
            height,
        })
    }
}

impl ScreenCapture for PlatformCapture {
    fn capture(&mut self) -> Result<Frame> {
        let image = self.monitor.capture_image().map_err(|e| anyhow::anyhow!("Capture error: {:?}", e))?;
        let width = image.width();
        let height = image.height();
        let data = image.into_raw();

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Ok(Frame {
            data,
            width,
            height,
            timestamp_ms,
        })
    }

    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
}

/// Create platform-appropriate screen capture instance
pub fn create_capture() -> Result<Box<dyn ScreenCapture>> {
    Ok(Box::new(PlatformCapture::new()?))
}
