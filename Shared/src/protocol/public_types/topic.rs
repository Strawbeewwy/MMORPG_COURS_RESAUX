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
pub const TOPIC_ID_LEN: usize = size_of::<u32>();
const TOPIC_HEADER_LEN: usize = size_of::<u8>() + TOPIC_ID_LEN;
const TOPIC_PADDING_LEN: usize = TOPIC_LEN - TOPIC_HEADER_LEN;


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
    pub fn to_string(&self) -> String {
        match self {
            Topic::Global => "global".to_string(),
            Topic::Chat => "chat".to_string(),
            Topic::Zone(id) => format!("sector_{:04}", id),
            Topic::ShardInstance(id) => format!("shard_{:02}", id.0),
        }
    }
}
impl BinaryEncode for Topic {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            Topic::Global => {
                write_u8(output, TOPIC_GLOBAL);
                write_u32(output, 0);
            }
            Topic::Chat => {
                write_u8(output, TOPIC_CHAT);
                write_u32(output, 0);
            }
            Topic::Zone(id) => {
                write_u8(output, TOPIC_ZONE);
                write_u32(output, *id);
            }
            Topic::ShardInstance(id) => {
                write_u8(output, TOPIC_SHARD_INSTANCE);
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
        let id = read_u32(input)?;
        let padding = read_exact(input, TOPIC_PADDING_LEN)?;

        if !padding.iter().all(|byte| *byte == 0) {
            anyhow::bail!("topic contains non-zero padding bytes");
        }

        match kind {
            TOPIC_GLOBAL => {
                if id != 0 {
                    anyhow::bail!("Global topic must not contain an id");
                }

                Ok(Topic::Global)
            }
            TOPIC_CHAT => {
                if id != 0 {
                    anyhow::bail!("Chat topic must not contain an id");
                }

                Ok(Topic::Chat)
            }
            TOPIC_ZONE => Ok(Topic::Zone(id)),
            TOPIC_SHARD_INSTANCE => Ok(Topic::ShardInstance(ShardId(id))),
            unknown => anyhow::bail!("unknown topic kind: 0x{unknown:02x}"),
        }
    }
}
