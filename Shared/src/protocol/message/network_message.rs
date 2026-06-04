use crate::protocol::message::config::CLIENT_INPUT_LEN;
use crate::protocol::public_types::topic::*;
use crate::protocol::{ClientId, EntityId, NetVec2, Username};
use crate::protocol::game::entity::EntityState;


#[derive(Debug, Clone)]
pub enum NetworkMessage {
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
        client_id: ClientId,
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
    RegisterClient {// from broker to shard
        client_id: ClientId,
        username: Username,
    },
    RegisterSpatialService,//from spatial to broker
    ClientHello {// from client to broker
        username: Username,
    },
    ClientAccepted { // from broker to client
        client_id: ClientId,
    },
    PositionUpdate { //from shard to broker then to spatial
        client_id: ClientId,
        position: NetVec2,
    },
    HandoffRequest {//from spatial to broker then to shard
        entity_id: EntityId,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState
    },
    HandoffAccepted {//from shard to broker then to spatial
        entity_id: EntityId,
    },
    HandoffRejected { // from shard to broker then to spatial
        entity_id: EntityId,
    },
    GhostUpdate { // from shard to broker to another shard
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
    },
    HandoffCompleted {// from spatial to broker then to shard
        entity_id: EntityId,
    },
}