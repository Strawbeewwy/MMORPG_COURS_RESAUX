use shared::protocol::public_types::*;
use crate::config::ServerConfig;
use crate::world::state::SharedEntityRegistry;

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