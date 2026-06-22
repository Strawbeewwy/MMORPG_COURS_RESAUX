use std::hash::{Hash, Hasher};
use crate::protocol::{EntityId, ClientId};
use crate::protocol::utils::utils::{
    BinaryDecode,
    BinaryEncode,
    read_exact,
    read_u8,
    read_u32,
    write_u8,
    write_u32,
};

pub const TOPIC_LEN: usize = 32;
const TOPIC_GLOBAL: u8 = 0x01;
const TOPIC_CHAT: u8 = 0x02;
const TOPIC_ZONE: u8 = 0x03;
const TOPIC_SHARD_INSTANCE: u8 = 0x04;
const TOPIC_ENTITY: u8 = 0x05;
const TOPIC_CLIENT: u8 = 0x06;
pub const TOPIC_ID_LEN: usize = size_of::<u32>();
const TOPIC_HEADER_LEN: usize = size_of::<u8>() + TOPIC_ID_LEN;
const TOPIC_PADDING_LEN: usize = TOPIC_LEN - TOPIC_HEADER_LEN;


#[derive(Debug, Clone, Copy, PartialEq, Eq,
    PartialOrd, Ord, Hash, Default)]
pub struct ShardId(pub u32);

#[derive(Debug, Clone, Copy, Eq,)]
pub enum Topic {
    Global{
        id: u32
    },//NOT USED
    Chat{
        id: u32
    },//NOT USED
    Zone{
        id: u32
    },//NOT USED
    ShardInstance{
        id: ShardId
    },
    Entity{
        id: EntityId,
    },
    Client{
        id: ClientId,
    },
}

impl PartialEq for Topic{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Topic::Entity { id: id1}, Topic::Entity { id: id2 }) => id1 == id2,
            (Topic::Global { id: id1 }, Topic::Global { id: id2 }) => id1 == id2,
            (Topic::Chat { id: id1 }, Topic::Chat { id: id2 }) => id1 == id2,
            (Topic::Zone { id: id1 }, Topic::Zone { id: id2 }) => id1 == id2,
            (Topic::ShardInstance { id: id1 }, Topic::ShardInstance { id: id2 }) => id1 == id2,
            (Topic::Client { id: id1 }, Topic::Client { id: id2 }) => id1 == id2,
            _ => false,
        }
    }
}

impl Hash for Topic {
    fn hash<H: Hasher>(&self, id_to_h: &mut H) {
        match self {
            Topic::Entity { id } => {
                id.hash(id_to_h);
            },
            Topic::Global { id } => {
                id.hash(id_to_h);
            }
            Topic::Chat { id } => {
                id.hash(id_to_h);
            }
            Topic::Zone { id } => {
                id.hash(id_to_h);
            }
            Topic::ShardInstance { id } => {
                id.hash(id_to_h);
            }
            Topic::Client { id } => {
                id.hash(id_to_h);
            }
        }

    }
}
impl Topic {

    pub fn get_id_as_u32(&self) ->u32{
        match self {
            Topic::Entity { id } => id.0,
            Topic::Chat { id } => *id,
            Topic::Zone { id } => *id,
            Topic::ShardInstance { id } => id.0,
            Topic::Global { id } => *id,
            Topic::Client { id } => id.0,
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Topic::Global {..}=> "global".to_string(),
            Topic::Chat{..} => "chat".to_string(),
            Topic::Zone{id} => format!("sector_{}", id),
            Topic::ShardInstance{id} => format!("shard_{}", id.0),
            Topic::Entity { id } => {format!("entity_:{}", id.0)},
            Topic::Client { id } => {format!("client_{}", id.0)},
        }
    }
}
impl BinaryEncode for Topic {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Topic::Global {id}=> {
                write_u8(output, TOPIC_GLOBAL);
                write_u32(output, *id);
            }
            Topic::Chat{id} => {
                write_u8(output, TOPIC_CHAT);
                write_u32(output, *id);
            }
            Topic::Zone{id} => {
                write_u8(output, TOPIC_ZONE);
                write_u32(output, *id);
            }
            Topic::ShardInstance{id} => {
                write_u8(output, TOPIC_SHARD_INSTANCE);
                write_u32(output, id.0);
            }
            Topic::Entity { id } => {
                write_u8(output, TOPIC_ENTITY);
                write_u32(output, id.0);
            }
            Topic::Client { id } => {
                write_u8(output, TOPIC_CLIENT);
                write_u32(output, id.0);
            }
        }

        output.extend_from_slice(&[0_u8; TOPIC_PADDING_LEN]);

        Ok(())
    }
}

impl BinaryDecode for Topic {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let kind = read_u8(input)?;
        let r_id = read_u32(input)?;
        let padding = read_exact(input, TOPIC_PADDING_LEN)?;


        if !padding.iter().all(|byte| *byte == 0) {
            anyhow::bail!("topic contains non-zero padding bytes");
        }

        match kind {
            TOPIC_GLOBAL => {
                Ok(Topic::Global{id:r_id})
            }
            TOPIC_CHAT => {
                Ok(Topic::Chat{id:r_id})
            }
            TOPIC_ZONE => Ok(Topic::Zone{id:r_id}),
            TOPIC_SHARD_INSTANCE => Ok(Topic::ShardInstance{
                id: ShardId(r_id)
            }),
            TOPIC_ENTITY => Ok(Topic::Entity{
                id: EntityId(r_id)
            }),
            TOPIC_CLIENT => Ok(Topic::Client{
                id: ClientId(r_id)
            }),
            unknown => anyhow::bail!("unknown topic kind: 0x{unknown:02x}"),
        }
    }
}
