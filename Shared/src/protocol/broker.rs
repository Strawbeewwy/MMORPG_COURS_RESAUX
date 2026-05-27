/// Binary little-endian broker protocol messages (TP2 Part 1).
///
/// Tags: Subscribe=0x01, Unsubscribe=0x02, Publish=0x03, Broadcast=0x04, ClientInput=0x05
use anyhow::{bail, Context, Result};

pub const TAG_SUBSCRIBE: u8 = 0x01;
pub const TAG_UNSUBSCRIBE: u8 = 0x02;
pub const TAG_PUBLISH: u8 = 0x03;
pub const TAG_BROADCAST: u8 = 0x04;
pub const TAG_CLIENT_INPUT: u8 = 0x05;

/// Fixed-size topic identifier — shard id encoded as "shard:N" zero-padded to 32 bytes.
pub type Topic = [u8; 32];

/// Build a topic bytes array from a shard id (e.g. shard_id=0 → "shard:0\0…").
pub fn topic_for_shard(shard_id: u32) -> Topic {
    let mut topic = [0u8; 32];
    let label = format!("shard:{shard_id}");
    let bytes = label.as_bytes();
    let len = bytes.len().min(32);
    topic[..len].copy_from_slice(&bytes[..len]);
    topic
}

/// Sent by the spatial service to the broker to subscribe a client to a shard topic.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscribe {
    pub client_id: u32,
    pub topic: Topic,
}

impl Subscribe {
    pub fn new(client_id: u32, shard_id: u32) -> Self {
        Self { client_id, topic: topic_for_shard(shard_id) }
    }

    /// Serialise to wire bytes: tag(1) | client_id(4) | topic(32) = 37 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(37);
        buf.push(TAG_SUBSCRIBE);
        buf.extend_from_slice(&self.client_id.to_le_bytes());
        buf.extend_from_slice(&self.topic);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 37 { bail!("Subscribe too short: {} bytes", data.len()); }
        if data[0] != TAG_SUBSCRIBE { bail!("wrong tag for Subscribe: 0x{:02x}", data[0]); }
        let client_id = u32::from_le_bytes(data[1..5].try_into().context("client_id")?);
        let topic: Topic = data[5..37].try_into().context("topic")?;
        Ok(Self { client_id, topic })
    }
}

/// Sent by the spatial service to the broker to unsubscribe a client from a shard topic.
#[derive(Debug, Clone, PartialEq)]
pub struct Unsubscribe {
    pub client_id: u32,
    pub topic: Topic,
}

impl Unsubscribe {
    pub fn new(client_id: u32, shard_id: u32) -> Self {
        Self { client_id, topic: topic_for_shard(shard_id) }
    }

    /// Serialise to wire bytes: tag(1) | client_id(4) | topic(32) = 37 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(37);
        buf.push(TAG_UNSUBSCRIBE);
        buf.extend_from_slice(&self.client_id.to_le_bytes());
        buf.extend_from_slice(&self.topic);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 37 { bail!("Unsubscribe too short: {} bytes", data.len()); }
        if data[0] != TAG_UNSUBSCRIBE { bail!("wrong tag for Unsubscribe: 0x{:02x}", data[0]); }
        let client_id = u32::from_le_bytes(data[1..5].try_into().context("client_id")?);
        let topic: Topic = data[5..37].try_into().context("topic")?;
        Ok(Self { client_id, topic })
    }
}

/// Sent by a shard to the broker to broadcast state to subscribed clients.
#[derive(Debug, Clone, PartialEq)]
pub struct Publish {
    pub topic: Topic,
    pub payload: Vec<u8>,
}

impl Publish {
    /// Serialise: tag(1) | topic(32) | payload_len(2) | payload.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(35 + self.payload.len());
        buf.push(TAG_PUBLISH);
        buf.extend_from_slice(&self.topic);
        buf.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 35 { bail!("Publish too short: {} bytes", data.len()); }
        if data[0] != TAG_PUBLISH { bail!("wrong tag for Publish: 0x{:02x}", data[0]); }
        let topic: Topic = data[1..33].try_into().context("topic")?;
        let payload_len = u16::from_le_bytes(data[33..35].try_into().context("payload_len")?) as usize;
        if data.len() < 35 + payload_len { bail!("Publish payload truncated"); }
        Ok(Self { topic, payload: data[35..35 + payload_len].to_vec() })
    }
}

/// Sent by the broker to connected clients.
#[derive(Debug, Clone, PartialEq)]
pub struct Broadcast {
    pub payload: Vec<u8>,
}

impl Broadcast {
    /// Serialise: tag(1) | payload_len(2) | payload.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3 + self.payload.len());
        buf.push(TAG_BROADCAST);
        buf.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 3 { bail!("Broadcast too short: {} bytes", data.len()); }
        if data[0] != TAG_BROADCAST { bail!("wrong tag for Broadcast: 0x{:02x}", data[0]); }
        let payload_len = u16::from_le_bytes(data[1..3].try_into().context("payload_len")?) as usize;
        if data.len() < 3 + payload_len { bail!("Broadcast payload truncated"); }
        Ok(Self { payload: data[3..3 + payload_len].to_vec() })
    }
}

/// Sent by a client through the broker — relayed by the broker to the appropriate shard.
#[derive(Debug, Clone, PartialEq)]
pub struct ClientInput {
    pub client_id: u32,
    pub input: [u8; 16],
}

impl ClientInput {
    /// Serialise: tag(1) | client_id(4) | input(16) = 21 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(21);
        buf.push(TAG_CLIENT_INPUT);
        buf.extend_from_slice(&self.client_id.to_le_bytes());
        buf.extend_from_slice(&self.input);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 21 { bail!("ClientInput too short: {} bytes", data.len()); }
        if data[0] != TAG_CLIENT_INPUT { bail!("wrong tag for ClientInput: 0x{:02x}", data[0]); }
        let client_id = u32::from_le_bytes(data[1..5].try_into().context("client_id")?);
        let input: [u8; 16] = data[5..21].try_into().context("input")?;
        Ok(Self { client_id, input })
    }
}

