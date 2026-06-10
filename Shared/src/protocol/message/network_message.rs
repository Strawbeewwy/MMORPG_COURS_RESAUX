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
    UnregisterClient {
        client_id: ClientId,
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
        start: u32,
        count: u32,
    },
    // a shard sends a position update to the spatial
    PositionUpdate {
        entity_id: EntityId,
        position: NetVec2,
    },
    RegisterEntity{
        entity_id: EntityId,
        client_id: ClientId, // ClientId(0) = non-player entity
        position: NetVec2,
    },
    UnregisterEntity{
        entity_id: EntityId,
    },
    HandoffStart{//the spatial sends a handoff start to the source shard
        entity_id: EntityId,
        source: ShardId,
        destination: ShardId,
    },
    HandoffRequest { // after the source shard receives a handoff start, it sends a handoff request to the destination shard
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState,
    },

    HandoffAccepted { // the destination shard sends a handoff accepted to the source shard
        entity_id: EntityId,
    },
    HandoffRejected { // the destination shard sends a handoff rejected to the source shard
        entity_id: EntityId,
    },
    GhostUpdate { // the source shard sends a ghost update to the destination shard
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
    },
    HandoffCompleted { //the source shard sends a handoff completed to the destination shard
        entity_id: EntityId,
    },
}