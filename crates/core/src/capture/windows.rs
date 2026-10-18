// crates/core/src/capture/windows.rs
// Windows DXGI Desktop Duplication API — hardware accelerated screen capture
// Captures at 60fps with minimal CPU usage
// Works with DirectX games, protected content (configurable), multiple monitors

use super::{Frame, ScreenCapture};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use windows_capture::{
    capture::{CaptureControl, GraphicsCaptureSession},
    frame::Frame as WinFrame,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
    window::Window,
};

pub struct WindowsCapture {
    width: u32,
    height: u32,
    // Channel to receive frames from the capture thread
    frame_rx: std::sync::mpsc::Receiver<Vec<u8>>,
    _capture_control: CaptureControl<CaptureHandler, anyhow::Error>,
}

#[cfg(target_os = "windows")]
struct CaptureHandler {
    frame_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
}

#[cfg(target_os = "windows")]
impl windows_capture::capture::WindowsCaptureHandler for CaptureHandler {
    type Flags = std::sync::mpsc::SyncSender<Vec<u8>>;
    type Error = anyhow::Error;

    fn new(tx: Self::Flags) -> Result<Self, Self::Error> {
        Ok(Self { frame_tx: tx })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut WinFrame,
        _: CaptureControl<Self, Self::Error>,
    ) -> Result<(), Self::Error> {
        // Get raw BGRA buffer from GPU
        let buffer = frame.buffer()?;
        let raw_data = buffer.as_raw_nopadding_buffer()?.to_vec();
        // Non-blocking send — drop frame if receiver is slow (prevents backpressure)
        let _ = self.frame_tx.try_send(raw_data);
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl WindowsCapture {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let primary_monitor = Monitor::primary()?;
            let width = primary_monitor.width()?;
            let height = primary_monitor.height()?;

            // Bounded channel — hold max 2 frames to minimize latency
            let (tx, rx) = std::sync::mpsc::sync_channel(2);

            let settings = Settings::new(
                primary_monitor,
                CursorCaptureSettings::WithCursor,  // Show remote cursor
                DrawBorderSettings::WithoutBorder,
                ColorFormat::Bgra8,
                tx,
            );

            let control = GraphicsCaptureSession::start(settings)?;

            Ok(Self {
                width,
                height,
                frame_rx: rx,
                _capture_control: control,
            })
        }
        #[cfg(not(target_os = "windows"))]
        anyhow::bail!("WindowsCapture only available on Windows")
    }
}

impl ScreenCapture for WindowsCapture {
    fn capture(&mut self) -> Result<Frame> {
        // Block until next frame arrives (60fps = ~16ms wait)
        let data = self.frame_rx.recv()?;
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

