// crates/core/src/input/mod.rs
// Cross-platform keyboard and mouse input simulation
// Used by HOST to replay client's input events

use anyhow::Result;
use enigo::{
    Button, Coordinate, Direction::{Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use crate::protocol::MessageType;

/// Input simulator — runs on HOST to replay remote input
pub struct InputSimulator {
    enigo: Enigo,
    screen_width: f64,
    screen_height: f64,
}

impl InputSimulator {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to create input simulator: {}", e))?;

        Ok(Self {
            enigo,
            screen_width: screen_width as f64,
            screen_height: screen_height as f64,
        })
    }

    /// Process an incoming input event from the client
    pub fn handle_event(&mut self, event: &MessageType) -> Result<()> {
        match event {
            MessageType::MouseMove { x, y } => {
                // Convert normalized (0.0-1.0) to absolute pixels
                let abs_x = (x * self.screen_width) as i32;
                let abs_y = (y * self.screen_height) as i32;
                self.enigo.move_mouse(abs_x, abs_y, Coordinate::Abs)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }

            MessageType::MouseButton { button, pressed, x, y } => {
                // Move to position first
                let abs_x = (x * self.screen_width) as i32;
                let abs_y = (y * self.screen_height) as i32;
                self.enigo.move_mouse(abs_x, abs_y, Coordinate::Abs)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let btn = match button {
                    crate::protocol::MouseButton::Left => Button::Left,
                    crate::protocol::MouseButton::Right => Button::Right,
                    crate::protocol::MouseButton::Middle => Button::Middle,
                };

                let direction = if *pressed { Press } else { Release };
                self.enigo.button(btn, direction)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }

            MessageType::MouseScroll { delta_x, delta_y } => {
                self.enigo.scroll(*delta_y as i32, enigo::Axis::Vertical)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                if *delta_x != 0.0 {
                    self.enigo.scroll(*delta_x as i32, enigo::Axis::Horizontal)
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                }
            }

            MessageType::KeyEvent { key, pressed, modifiers: _ } => {
                // Convert key code to enigo key
                let enigo_key = map_keycode(*key);
                let direction = if *pressed { Press } else { Release };
                self.enigo.key(enigo_key, direction)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            }

            _ => {} // Non-input messages ignored here
        }
        Ok(())
    }
}

/// Map USB HID key codes to enigo Key enum
fn map_keycode(code: u32) -> Key {
    match code {
        // Letters A-Z
        65..=90 => {
            let c = char::from_u32(code + 32).unwrap_or('a'); // lowercase
            Key::Unicode(c)
        }
        // Digits 0-9
        48..=57 => Key::Unicode(char::from_u32(code).unwrap_or('0')),
        // Special keys
        8   => Key::Backspace,
        9   => Key::Tab,
        13  => Key::Return,
        27  => Key::Escape,
        32  => Key::Space,
        37  => Key::LeftArrow,
        38  => Key::UpArrow,
        39  => Key::RightArrow,
        40  => Key::DownArrow,
        46  => Key::Delete,
        112 => Key::F1,
        113 => Key::F2,
        114 => Key::F3,
        115 => Key::F4,
        116 => Key::F5,
        117 => Key::F6,
        118 => Key::F7,
        119 => Key::F8,
        120 => Key::F9,
        121 => Key::F10,
        122 => Key::F11,
        123 => Key::F12,
        16  => Key::Shift,
        17  => Key::Control,
        18  => Key::Alt,
        91  => Key::Meta,  // Windows/Command key
        _   => Key::Unicode(char::from_u32(code).unwrap_or('\0')),
    }
}

