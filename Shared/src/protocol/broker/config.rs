pub const TAG_SUBSCRIBE: u8 = 0x01;
pub const TAG_UNSUBSCRIBE: u8 = 0x02;
pub const TAG_PUBLISH: u8 = 0x03;
pub const TAG_BROADCAST: u8 = 0x04;
pub const TAG_CLIENT_INPUT: u8 = 0x05;
pub const TAG_REGISTER_SHARD: u8 = 0x06;
pub const TAG_REGISTER_SPATIAL_SERVICE: u8 = 0x07;
pub const TAG_CLIENT_HELLO: u8 = 0x08;
pub const TAG_CLIENT_ACCEPTED: u8 = 0x09;
pub const TAG_POSITION_UPDATE: u8 = 0x10;
/// Sent by a shard to the SpatialService immediately after connecting,
/// to register its identity (shard_id → GameConnection mapping).
pub const TAG_SHARD_REGISTER: u8 = 0x11;
/// Sent by the SpatialService to the destination shard to initiate a client handoff.
pub const TAG_HANDOFF_REQUEST: u8 = 0x12;
/// Sent by the destination shard back to the SpatialService to confirm it accepted the client.
pub const TAG_HANDOFF_ACK: u8 = 0x13;
pub const TAG_LEN: usize = 1;
pub const MAX_PAYLOAD_LEN: usize = u16::MAX as usize;
pub const CLIENT_INPUT_LEN: usize = 16;