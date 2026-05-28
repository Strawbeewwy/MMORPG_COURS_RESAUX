pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, TOPIC_LEN, ClientId, Topic, BrokerMessage,
};


pub fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub fn read_topic(bytes: &[u8]) -> Topic {
    let mut topic = [0_u8; TOPIC_LEN];
    topic.copy_from_slice(bytes);
    topic
}

/// Build a Topic from a shard id (e.g. 0 → "shard:0").
/// Uses a small inline buffer to avoid heap allocation on every call.
pub fn topic_for_shard(shard_id: u32) -> Topic {
    let mut topic = [0u8; TOPIC_LEN];
    // Write "shard:" prefix then the decimal digits directly into the buffer.
    let prefix = b"shard:";
    topic[..prefix.len()].copy_from_slice(prefix);
    let mut n = shard_id;
    let mut digits = [0u8; 10]; // u32::MAX is 10 digits
    let mut len = 0usize;
    if n == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        digits[..len].reverse();
    }
    topic[prefix.len()..prefix.len() + len].copy_from_slice(&digits[..len]);
    topic
}

pub fn topic_from_str(value: &str) -> Topic {
    let mut topic = [0_u8; TOPIC_LEN];
    let bytes = value.as_bytes();
    let len = bytes.len().min(TOPIC_LEN);

    topic[..len].copy_from_slice(&bytes[..len]);

    topic
}

pub fn topic_to_string(topic: &Topic) -> String {
    let len = topic
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(TOPIC_LEN);

    String::from_utf8_lossy(&topic[..len]).to_string()
}