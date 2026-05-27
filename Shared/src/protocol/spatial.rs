/// Binary little-endian spatial protocol messages (TP2 Part 2).
///
/// PositionUpdate tag = 0x10, sent by shards to the spatial service.
/// CrossingAlert is internal (Bevy event) — triggers handoff logic in Part 3.
use anyhow::{bail, Context, Result};

pub const TAG_POSITION_UPDATE: u8 = 0x10;

/// Sent by a shard to the spatial service on every player move.
/// Wire format: tag(1) | client_id(4) | x(4) | y(4) = 13 bytes, all little-endian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionUpdate {
    pub client_id: u32,
    pub x: f32,
    pub y: f32,
}

impl PositionUpdate {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(13);
        buf.push(TAG_POSITION_UPDATE);
        buf.extend_from_slice(&self.client_id.to_le_bytes());
        buf.extend_from_slice(&self.x.to_le_bytes());
        buf.extend_from_slice(&self.y.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 13 {
            bail!("PositionUpdate too short: {} bytes", data.len());
        }
        if data[0] != TAG_POSITION_UPDATE {
            bail!("wrong tag for PositionUpdate: 0x{:02x}", data[0]);
        }
        let client_id = u32::from_le_bytes(data[1..5].try_into().context("client_id")?);
        let x = f32::from_le_bytes(data[5..9].try_into().context("x")?);
        let y = f32::from_le_bytes(data[9..13].try_into().context("y")?);
        Ok(Self { client_id, x, y })
    }
}

/// Internal Bevy event — emitted when a client is near a shard boundary.
/// Consumed by the crossing system; will trigger HandoffRequest in Part 3.
#[derive(Debug, Clone)]
pub struct CrossingAlert {
    pub client_id: u32,
    /// All distinct shard ids covering the margin area around the client position.
    pub shards: Vec<u32>,
}

