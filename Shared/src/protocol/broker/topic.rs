

pub const TOPIC_LEN: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ShardId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    Global,//NOT USED
    Chat,//NOT USED
    Zone(u32),//NOT USED
    ShardInstance(ShardId),
}

impl Topic {
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];

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
                bytes[prefix.len()..prefix.len() + 4].copy_from_slice(&id_bytes);
            }
            Topic::ShardInstance(id) => {
                let prefix = b"shard_";
                bytes[..prefix.len()].copy_from_slice(prefix);

                let id_bytes = id.0.to_le_bytes();
                bytes[prefix.len()..prefix.len() + 2].copy_from_slice(&id_bytes);
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

impl TryFrom<[u8; 32]> for Topic {
    type Error = &'static str;

    fn try_from(bytes: [u8; 32]) -> Result<Self, Self::Error> {
        if bytes.starts_with(b"global") {
            Ok(Topic::Global)
        } else if bytes.starts_with(b"chat") {
            Ok(Topic::Chat)
        } else if bytes.starts_with(b"sector_") {
            let id_bytes = bytes[7..11].try_into().map_err(|_| "Format d'ID invalide")?;
            let id = u32::from_le_bytes(id_bytes);
            Ok(Topic::Zone(id))
        } else if bytes.starts_with(b"shard_") {
            let id_bytes = bytes[6..8].try_into().map_err(|_| "Format d'ID invalide")?;
            let id = u32::from_le_bytes(id_bytes);
            Ok(Topic::ShardInstance(ShardId(id)))
        } else {
            Err("Topic inconnu ou malformé")
        }
    }
}

pub fn read_topic(bytes: &[u8]) -> Topic {

    let mut topic_bytes = [0u8; TOPIC_LEN];

    topic_bytes[..bytes.len()].copy_from_slice(bytes);

    let topic = Topic::try_from(topic_bytes);

    topic.unwrap()
}

