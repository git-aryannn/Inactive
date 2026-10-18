// crates/core/src/capture/macos.rs
// macOS ScreenCaptureKit wrapper via `scap` crate
// Requires user permission: System Settings → Privacy → Screen Recording
// Apple Silicon (M1/M2/M3) optimized — uses Metal GPU pipeline

use super::{Frame, ScreenCapture};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::{Frame as ScapFrame, FrameType, VideoFrame},
};

pub struct MacosCapture {
    #[cfg(target_os = "macos")]
    capturer: Capturer,
    width: u32,
    height: u32,
}

impl MacosCapture {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // Check and request screen recording permission
            if !scap::has_permission() {
                // Triggers macOS permission dialog
                scap::request_permission();
                if !scap::has_permission() {
                    anyhow::bail!(
                        "Screen Recording permission denied. \
                         Please enable in System Settings → Privacy & Security → Screen Recording"
                    );
                }
            }

            let options = Options {
                fps: 60,
                output_resolution: Resolution::Captured,  // Native resolution
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
        #[cfg(not(target_os = "macos"))]
        anyhow::bail!("MacosCapture only available on macOS")
    }
}

impl ScreenCapture for MacosCapture {
    fn capture(&mut self) -> Result<Frame> {
        #[cfg(target_os = "macos")]
        {
            let raw_frame = self.capturer.get_next_frame()
                .map_err(|e| anyhow::anyhow!("Capture error: {:?}", e))?;
            let data = match raw_frame {
                ScapFrame::Video(VideoFrame::BGRA(f)) => f.data,
                _ => anyhow::bail!("Unexpected frame format"),
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
        #[cfg(not(target_os = "macos"))]
        anyhow::bail!("MacosCapture only available on macOS")
    }

    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
}

impl Drop for MacosCapture {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        self.capturer.stop_capture();
    }
}
