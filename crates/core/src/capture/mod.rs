// crates/core/src/capture/mod.rs
// Platform-specific screen capture abstraction

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;

use anyhow::Result;

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

/// Create platform-appropriate screen capture instance
pub fn create_capture() -> Result<Box<dyn ScreenCapture>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsCapture::new()?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacosCapture::new()?))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        anyhow::bail!("Unsupported platform for screen capture")
    }
}

