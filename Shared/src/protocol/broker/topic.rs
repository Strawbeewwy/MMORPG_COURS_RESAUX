use crate::protocol::broker::utils::read_u32_le;

pub const TOPIC_LEN: usize = 32;
pub const SHARD_ID_LEN: usize = size_of::<u32>();
pub const ZONE_ID_LEN: usize = size_of::<u32>();

#[derive(Debug, Clone, Copy, PartialEq, Eq,
    PartialOrd, Ord, Hash, Default)]
pub struct ShardId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    Global,//NOT USED
    Chat,//NOT USED
    Zone(u32),//NOT USED
    ShardInstance(ShardId),
}

impl Topic {
    pub fn to_bytes(&self) -> [u8; TOPIC_LEN] {
        let mut bytes = [0u8; TOPIC_LEN];

        match self {
            Topic::Global => {
                let name = b"global";
                bytes[..name.len()].copy_from_slice(name);
            }
            Topic::Chat => {
                let name = b"chat";
                bytes[..name.len()].copy_from_slice(name);
            }
            Topic::Zone(id) => {

                let prefix = b"sector_";
                bytes[..prefix.len()].copy_from_slice(prefix);

                let id_bytes = id.to_le_bytes();
                bytes[prefix.len()..prefix.len() + ZONE_ID_LEN].copy_from_slice(&id_bytes);
            }
            Topic::ShardInstance(id) => {
                let prefix = b"shard_";
                bytes[..prefix.len()].copy_from_slice(prefix);

                let id_bytes = id.0.to_le_bytes();
                bytes[prefix.len()..prefix.len() + SHARD_ID_LEN].copy_from_slice(&id_bytes);
            }
        }

        bytes
    }

    pub fn to_string(&self) -> String {
        match self {
            Topic::Global => "global".to_string(),
            Topic::Chat => "chat".to_string(),
            Topic::Zone(id) => format!("sector_{:04}", id),
            Topic::ShardInstance(id) => format!("shard_{:02}", id.0),
        }
    }
}

impl TryFrom<[u8; TOPIC_LEN]> for Topic {
    type Error = &'static str;

    fn try_from(bytes: [u8; TOPIC_LEN]) -> Result<Self, Self::Error> {

       /*
        this function checks if the bytes starting from a determined index are all zero
        this prevents reading past the end of the buffer and ensures the topic is properly formatted
        it also prevents any trailing data from being read as part of the topic
        */
        let is_zero_padded = |start_idx: usize| -> bool {

            bytes.get(start_idx..).map_or(false, |slice| slice.iter().all(|&b| b == 0))
        };

        if bytes.starts_with(b"global") && is_zero_padded(b"global".len()) {
            Ok(Topic::Global)
        } else if bytes.starts_with(b"chat") && is_zero_padded(b"chat".len()) {
            Ok(Topic::Chat)
        } else if bytes.starts_with(b"sector_") {
            let id_start = b"sector_".len();
            let id_end = id_start + size_of::<u32>();

            if let Some(id_slice) = bytes.get(id_start..id_end) {

                if is_zero_padded(id_end) {
                    let id = read_u32_le(id_slice);
                    return Ok(Topic::Zone(id));
                }
            }
            Err("Zone topic is malformed or contains trailing data")
        } else if bytes.starts_with(b"shard_") {
            let id_start = b"shard_".len();
            let id_end = id_start + size_of::<u32>();

            if let Some(id_slice) = bytes.get(id_start..id_end) {
                if is_zero_padded(id_end) {
                    let id = read_u32_le(id_slice);
                    return Ok(Topic::ShardInstance(ShardId(id)));
                }
            }
            Err("ShardInstance topic is malformed or contains trailing data")
        } else {
            Err("Topic unknown or malformed")
        }
    }
}

pub fn read_topic(bytes: &[u8]) -> Topic {

    let mut topic_bytes = [0u8; TOPIC_LEN];

    topic_bytes[..bytes.len()].copy_from_slice(bytes);

    let topic = Topic::try_from(topic_bytes);

    topic.unwrap()
}

