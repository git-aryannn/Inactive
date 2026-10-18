// crates/core/src/capture/mod.rs
// Cross-platform screen capture abstraction using scap (DXGI on Windows, ScreenCaptureKit on macOS)

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::{Frame as ScapFrame, FrameType, VideoFrame},
};

/// Raw BGRA frame from screen capture
pub struct Frame {
    /// Raw BGRA pixel data
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
    capturer: Capturer,
    width: u32,
    height: u32,
}

impl PlatformCapture {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            if !scap::has_permission() {
                scap::request_permission();
                if !scap::has_permission() {
                    anyhow::bail!(
                        "Screen Recording permission denied. \
                         Please enable in System Settings → Privacy & Security → Screen Recording"
                    );
                }
            }
        }

        let options = Options {
            fps: 60,
            output_resolution: Resolution::Captured,
            show_cursor: true,
            show_highlight: false,
            output_type: FrameType::BGRAFrame,
            ..Default::default()
        };

        let mut capturer = Capturer::build(options)
            .map_err(|e| anyhow::anyhow!("Failed to build capturer: {:?}", e))?;
        capturer.start_capture();

        // Get actual dimensions from first frame
        let first_frame = capturer.get_next_frame()
            .map_err(|e| anyhow::anyhow!("Failed to get first frame: {:?}", e))?;
        let (width, height) = match &first_frame {
            ScapFrame::Video(VideoFrame::BGRA(f)) => (f.width as u32, f.height as u32),
            _ => anyhow::bail!("Unexpected frame format"),
        };

        Ok(Self { capturer, width, height })
    }
}

impl ScreenCapture for PlatformCapture {
    fn capture(&mut self) -> Result<Frame> {
        let frame = self.capturer.get_next_frame()
            .map_err(|e| anyhow::anyhow!("Capture error: {:?}", e))?;

        let data = match frame {
            ScapFrame::Video(VideoFrame::BGRA(f)) => f.data,
            _ => anyhow::bail!("Expected BGRA video frame"),
        };

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Ok(Frame {
            data,
            width: self.width,
            height: self.height,
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
