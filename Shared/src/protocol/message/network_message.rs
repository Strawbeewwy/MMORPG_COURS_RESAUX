use crate::protocol::message::config::*;
use crate::protocol::public_types::*;
use crate::Username;

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Subscribe {//spatial subscribes a client to a topic
        client_id: ClientId,
        topic: Topic,
    },
    Unsubscribe {// spatial unsubscribes a client from a topic
        client_id: ClientId,
        topic: Topic
    },
    Publish {//a topic publishes a payload
        topic: Topic,
        payload_len: u16,
        payload: Vec<u8>,
    },
    Broadcast {//broker broadcast the payload
        payload_len: u16,
        payload: Vec<u8>,
    },
    ClientInput {// sent from client to it's owned shard
        client_id: ClientId,
        input: [u8; CLIENT_INPUT_LEN],
    },
    RegisterShard {// a shard registers itself with the broker
        shard_id: ShardId,
    },
    RegisterClient {
        client_id: ClientId,
        username: Username,
    },
    RegisterSpatialService,// a spatial service registers itself with the broker
    ClientHello {//first contact from client to broker
        username: Username,
    },
    ClientAccepted {// broker accepts client registration
        client_id: ClientId,
    },
    // a shard requests a block of entity ids to own
    // so a shard can instantiate entities without conflict
    RequestEntityIdBlock {
        shard_id: ShardId,
        count: u32,
    },
    //the spatial sends a block of ids to a shard
    EntityIdBlockAllocated {
        shard_id: ShardId,
        start: u32,
        count: u32,
    },
    // a shard sends a position update to the spatial
    PositionUpdate {
        entity_id: EntityId,
        position: NetVec2,
    },


    //TODO decide how the handoff needs to be done
    // if a client moves diagonally, how are we supposed
    // to handle it?
    HandoffRequest {
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState,
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