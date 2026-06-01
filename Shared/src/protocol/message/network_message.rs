use crate::protocol::message::config::CLIENT_INPUT_LEN;
use crate::protocol::public_types::topic::*;
use crate::protocol::{ClientId, EntityId, NetVec2, Username};
use crate::protocol::game::entity::EntityState;


#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Subscribe { //from spatial to utils
        client_id: ClientId,
        shard_id: ShardId,
    },
    Unsubscribe { // from spatial to utils
        client_id: ClientId,
        shard_id: ShardId,
    },
    Publish { // from shard to utils
        shard_id: ShardId,
        payload_len: u16,
        payload: Vec<u8>,
    },
    Broadcast { // from utils to client
        payload_len: u16,
        payload: Vec<u8>,
    },
    ClientInput { // from client to utils
        client_id: ClientId,
        input: [u8; CLIENT_INPUT_LEN],
    },
    RegisterShard {// from spatial to utils
        shard_id: ShardId,
    },
    RegisterClient {
        client_id: ClientId,
        username: Username,
    },
    RegisterSpatialService,//from spatial to utils
    ClientHello {
        username: Username,
    },
    ClientAccepted { // from utils to client
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