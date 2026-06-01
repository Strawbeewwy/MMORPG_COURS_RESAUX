use serde::{Deserialize, Serialize};
use crate::protocol::broker::config::CLIENT_INPUT_LEN;
use crate::protocol::broker::topic::*;
use crate::protocol::broker::utils::read_u32_le;
use crate::protocol::{EntityId, NetVec2, Username};
use crate::protocol::game::entity::EntityState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default , Hash, Serialize, Deserialize)]
pub struct ClientId(pub u32);

impl From<ClientId> for u32 {
    #[inline]
    fn from(client_id: ClientId) -> Self {
        client_id.0
    }
}

pub const CLIENT_ID_LEN: usize = size_of::<ClientId>();





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
    RegisterClient {
        client_id: ClientId,
        username: Username,
    },
    RegisterSpatialService,//from spatial to broker
    ClientHello {
        username: Username,
    },
    ClientAccepted { // from broker to client
        client_id: ClientId,
    },
    PositionUpdate { //from shard to spatial
        client_id: ClientId,
        position: NetVec2,
    },
    HandoffRequest {//
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState
    },
    HandoffAccepted {
        entity_id: EntityId,
    },
    HandoffRejected {
        entity_id: EntityId,
    },
    GhostUpdate {
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
    },
    HandoffCompleted {
        entity_id: EntityId,
    },
}