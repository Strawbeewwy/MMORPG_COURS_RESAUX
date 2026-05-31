
use crate::protocol::broker::config::CLIENT_INPUT_LEN;
use crate::protocol::broker::topic::*;
use crate::protocol::broker::utils::read_u32_le;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default , Hash)]
pub struct ClientId(pub u32);

impl From<ClientId> for u32 {
    #[inline]
    fn from(client_id: ClientId) -> Self {
        client_id.0
    }
}

pub const CLIENT_ID_LEN: usize = size_of::<ClientId>();

pub fn read_client_id(bytes: &[u8]) -> ClientId {

    let mut client_id_bytes = [0_u8; CLIENT_ID_LEN];

    client_id_bytes[..bytes.len()].copy_from_slice(bytes);

    ClientId(read_u32_le(&client_id_bytes))
}



#[derive(Debug, Clone)]
pub enum BrokerMessage {
    Subscribe { //from spatial to broker
        client_id: ClientId,
        shard_id: ShardId,
    },
    Unsubscribe { // from spatial to broker
        client_id: ClientId,
        shard_id: ShardId,
    },
    Publish { // from shard to broker
        shard_id: ShardId,
        payload_len: u16,
        payload: Vec<u8>,
    },
    Broadcast { // from broker to client
        payload_len: u16,
        payload: Vec<u8>,
    },
    ClientInput { // from client to broker
        client_id: ClientId,
        input: [u8; CLIENT_INPUT_LEN],
    },
    RegisterShard {// from spatial to broker
        shard_id: ShardId,
    },
    RegisterSpatialService,//from spatial to broker
    ClientHello{ // from client to broker
        username: String,
    },
    ClientAccepted { // from broker to client
        client_id: ClientId,
    },
    PositionUpdate { //from shard to spatial
        client_id: ClientId,
        position: [f32; 2],
    },
    /// Sent by a shard to the SpatialService immediately after connecting.
    /// Registers shard_id ↔ GameConnection so HandoffRequest can be routed back.
    ShardRegister {
        shard_id: ShardId,
    },
    /// Sent by the SpatialService to the destination shard to initiate a handoff.
    HandoffRequest {
        client_id: ClientId,
        from_shard: ShardId,
        to_shard: ShardId,
    },
    /// Sent by the destination shard back to the SpatialService to confirm acceptance.
    HandoffAck {
        client_id: ClientId,
        to_shard: ShardId,
    },
}