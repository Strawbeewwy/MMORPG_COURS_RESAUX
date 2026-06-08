use crate::protocol::message::config::CLIENT_INPUT_LEN;
use crate::protocol::public_types::topic::*;
use crate::protocol::{ClientId, EntityId, EntityType, NetVec2, Username};
use crate::protocol::public_types::entity::EntityState;

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Subscribe {
        client_id: ClientId,
        topic: Topic,
    },
    Unsubscribe {
        client_id: ClientId,
        topic: Topic
    },
    Publish {
        topic: Topic,
        payload_len: u16,
        payload: Vec<u8>,
    },
    Broadcast {
        payload_len: u16,
        payload: Vec<u8>,
    },
    ClientInput {
        client_id: ClientId,
        input: [u8; CLIENT_INPUT_LEN],
    },
    RegisterShard {
        shard_id: ShardId,
    },
    RegisterClient {
        client_id: ClientId,
        username: Username,
    },
    RegisterSpatialService,
    ClientHello {
        username: Username,
    },
    ClientAccepted {
        client_id: ClientId,
    },

    RequestEntityIdBlock {
        shard_id: ShardId,
        count: u32,
    },
    EntityIdBlockAllocated {
        shard_id: ShardId,
        start: u32,
        count: u32,
    },

    PositionUpdate {
        entity_id: EntityId,
        position: NetVec2,
    },
    HandoffRequest {
        entity_id: EntityId,
        entity_type: EntityType,
        owner_client_id: Option<ClientId>,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState,
    },
    HandoffAccepted {
        entity_id: EntityId,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
    },
    HandoffRejected {
        entity_id: EntityId,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
    },
    GhostUpdate {
        entity_id: EntityId,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
        position: NetVec2,
        velocity: NetVec2,
    },
    HandoffCompleted {
        entity_id: EntityId,
        from_shard_id: ShardId,
        to_shard_id: ShardId,
    },
}