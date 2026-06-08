use shared::protocol::{EntityId, NetVec2, ShardId};
use shared::protocol::game::EntityState;
use crate::config::ServerConfig;
use crate::net::network_event::SharedEntityRegistry;

pub fn handle_handoff_request(
    config: &ServerConfig,
    registry: &SharedEntityRegistry,
    entity_id: EntityId,
    from_shard_id: ShardId,
    to_shard_id: ShardId,
    position: NetVec2,
    velocity: NetVec2,
    entity_state: EntityState,
){


}