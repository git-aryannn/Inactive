// crates/core/src/codec/mod.rs
// Video encoding and decoding interfaces

use anyhow::Result;

pub struct EncodedChunk {
    pub data: Vec<u8>,
    pub is_keyframe: bool,
    pub timestamp_ms: u64,
}

pub trait VideoEncoder: Send {
    fn encode(&mut self, bgra_data: &[u8], width: u32, height: u32) -> Result<EncodedChunk>;
}

pub trait VideoDecoder: Send {
    fn decode(&mut self, chunk: &[u8], width: u32, height: u32) -> Result<Vec<u8>>;
}

/// Fallback / Lightweight encoder (pass-through / simple intra compression)
pub struct RawFrameEncoder {
    timestamp: u64,
}

impl RawFrameEncoder {
    pub fn new() -> Self {
        Self { timestamp: 0 }
    }
}

impl Default for RawFrameEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoEncoder for RawFrameEncoder {
    fn encode(&mut self, bgra_data: &[u8], _width: u32, _height: u32) -> Result<EncodedChunk> {
        self.timestamp += 16; // approx 60fps step
        Ok(EncodedChunk {
            data: bgra_data.to_vec(),
            is_keyframe: true,
            timestamp_ms: self.timestamp,
        })
    }
}
