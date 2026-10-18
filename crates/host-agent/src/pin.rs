// crates/host-agent/src/pin.rs
// Durable & Persistent 9-digit Device PIN Manager
// PIN persists across disconnections, restarts, and sessions.
// Automatically saved to device configuration file.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PinConfig {
    pub pin: String,
    pub created_at: u64,
}

pub struct PinManager {
    current_pin: String,
    config_path: Option<PathBuf>,
}

impl PinManager {
    /// Load existing persistent PIN from disk, or generate and save a new durable one
    pub fn load_or_generate() -> Self {
        let config_path = Self::get_config_path();

        // 1. Try reading existing saved PIN
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<PinConfig>(&content) {
                let clean = cfg.pin.replace(['-', ' '], "");
                if clean.len() == 9 && clean.chars().all(|c| c.is_ascii_digit()) {
                    let formatted = format_clean_pin(&clean);
                    info!("Loaded persistent Device PIN from disk: {}", formatted);
                    return Self {
                        current_pin: formatted,
                        config_path: Some(config_path),
                    };
                }
            }
        }

        // 2. Generate new PIN and persist it
        let pin = Self::generate_random_pin();
        info!("Generated new persistent Device PIN: {}", pin);
        let mgr = Self {
            current_pin: pin,
            config_path: Some(config_path),
        };
        mgr.save();
        mgr
    }

    /// Create an in-memory PinManager with a specific PIN (useful for testing)
    pub fn new_with_pin(pin: &str) -> Self {
        let clean = pin.replace(['-', ' '], "");
        let formatted = format_clean_pin(&clean);
        Self {
            current_pin: formatted,
            config_path: None,
        }
    }

    /// Default constructor that loads or generates persistent PIN
    pub fn new() -> Self {
        Self::load_or_generate()
    }

    /// Manually regenerate a fresh PIN (e.g. user clicks "Reset PIN")
    pub fn regenerate(&mut self) -> String {
        self.current_pin = Self::generate_random_pin();
        self.save();
        info!("Device PIN regenerated and saved: {}", self.current_pin);
        self.current_pin.clone()
    }

    /// Validate entered PIN (handles dashes, spaces, case-insensitively)
    /// Durable: does NOT expire and does NOT self-destruct after one connection!
    pub fn validate(&self, entered: &str) -> PinValidation {
        let clean_entered = entered.replace(['-', ' '], "");
        let clean_current = self.current_pin.replace('-', "");

        if clean_entered.len() != 9 || !clean_entered.chars().all(|c| c.is_ascii_digit()) {
            return PinValidation::Invalid;
        }

        if clean_entered == clean_current {
            PinValidation::Valid
        } else {
            PinValidation::Invalid
        }
    }

    /// Get current PIN for display (formatted "123-456-789")
    pub fn current_pin_display(&self) -> &str {
        &self.current_pin
    }

    fn generate_random_pin() -> String {
        let pin: u32 = rand::thread_rng().gen_range(100_000_000..=999_999_999);
        let s = format!("{:09}", pin);
        format!("{}-{}-{}", &s[..3], &s[3..6], &s[6..])
    }

    fn save(&self) {
        if let Some(ref path) = self.config_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let cfg = PinConfig {
                pin: self.current_pin.clone(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                let _ = fs::write(path, json);
            }
        }
    }

    fn get_config_path() -> PathBuf {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Inactive");
        base_dir.join("device_pin.json")
    }
}

impl Default for PinManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format 9-digit string as "123-456-789"
fn format_clean_pin(clean: &str) -> String {
    if clean.len() == 9 {
        format!("{}-{}-{}", &clean[..3], &clean[3..6], &clean[6..])
    } else {
        clean.to_string()
    }
}

/// Format 9-digit number as "123-456-789"
pub fn format_pin(pin: u32) -> String {
    let s = format!("{:09}", pin);
    format!("{}-{}-{}", &s[..3], &s[3..6], &s[6..])
}

#[derive(Debug, PartialEq, Eq)]
pub enum PinValidation {
    Valid,
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_format() {
        assert_eq!(format_pin(123456789), "123-456-789");
        assert_eq!(format_pin(100000000), "100-000-000");
    }

    #[test]
    fn test_pin_validation() {
        let mgr = PinManager::new_with_pin("123-456-789");

        // Valid with dashes
        assert_eq!(mgr.validate("123-456-789"), PinValidation::Valid);
        // Valid without dashes
        assert_eq!(mgr.validate("123456789"), PinValidation::Valid);
        // Valid with spaces
        assert_eq!(mgr.validate("123 456 789"), PinValidation::Valid);
        // Invalid PIN
        assert_eq!(mgr.validate("999-999-999"), PinValidation::Invalid);

        // Crucial requirement: PIN is durable and does not expire or self-destruct after validation!
        // Reconnect 1
        assert_eq!(mgr.validate("123-456-789"), PinValidation::Valid);
        // Reconnect 2
        assert_eq!(mgr.validate("123-456-789"), PinValidation::Valid);
        // Reconnect 3
        assert_eq!(mgr.validate("123456789"), PinValidation::Valid);
    }
}
