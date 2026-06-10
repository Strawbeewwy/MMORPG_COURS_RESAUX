use bevy::prelude::*;
use shared::protocol::{EntityId, EntityState, EntityType, NetVec2, NetworkMessage, ShardId};
use crate::config::ServerConfig;
use crate::net::network_event::BrokerShardPeer;
use crate::world::{Ghost, PendingHandoff, Position, Velocity, NetworkEntityId, Authoritative};
use crate::world::entity::PromoteGhostEvent;
use crate::world::spawn_entity::SpawnGhostEntityEvent;
use crate::world::state::SharedEntityRegistry;

/// Source shard received HandoffStart → send HandoffRequest to broker (routed to dest).
pub fn handle_handoff_start_on_source(
    _config: &ServerConfig,
    broker: &mut BrokerShardPeer,
    _registry: &SharedEntityRegistry,
    entity_id: EntityId,
    destination: ShardId,
    position: NetVec2,
    velocity: NetVec2,
) {

    if let Err(e) = broker.send_message_to_broker(&NetworkMessage::HandoffRequest {
        entity_id,
        position,
        velocity,
        entity_state: EntityState::PendingHandoff,
    }) {
        tracing::error!("failed to send HandoffRequest to broker: {e:#}");
        return;
    }

    tracing::info!(
        "HandoffRequest sent: entity {} → dest shard {}",
        entity_id.0, destination.0
    );
}

/// Destination shard received HandoffRequest → spawn ghost entity, send HandoffAccepted.
pub fn handle_handoff_request_on_dest(
    broker: &mut BrokerShardPeer,
    spawn_ghosts: &mut MessageWriter<SpawnGhostEntityEvent>,
    registry: &SharedEntityRegistry,
    entity_id: EntityId,
    source_shard: ShardId,
    position: NetVec2,
    velocity: NetVec2,
) {
    // Avoid duplicate ghost spawning.
    if let Some((_, ent_reg)) = registry.try_lock() {
        if ent_reg.get_bevy_entity(&entity_id).is_some() {
            tracing::debug!("entity {} already exists on dest shard — skipping ghost spawn", entity_id.0);
        } else {
            spawn_ghosts.write(SpawnGhostEntityEvent {
                entity_id,
                entity_type: EntityType::Player,
                source_shard_id: source_shard,
                position: Vec2::new(position.x as f32, position.y as f32),
                velocity: Vec2::new(velocity.x as f32, velocity.y as f32),
            });
        }
    }

    if let Err(e) = broker.send_message_to_broker(&NetworkMessage::HandoffAccepted { entity_id }) {
        tracing::error!("failed to send HandoffAccepted for entity {}: {e:#}", entity_id.0);
        return;
    }

    tracing::info!("HandoffAccepted sent for entity {}", entity_id.0);
}

/// Source shard received HandoffAccepted → entity is now in ghost phase (still authoritative until HandoffCompleted).
pub fn handle_handoff_accepted_on_source(
    commands: &mut Commands,
    registry: &SharedEntityRegistry,
    entity_id: EntityId,
    destination: ShardId,
) {
    let Some((_, ent_reg)) = registry.try_lock() else {
        tracing::warn!("could not lock registry for HandoffAccepted entity {}", entity_id.0);
        return;
    };

    let Some(bevy_entity) = ent_reg.get_bevy_entity(&entity_id) else {
        tracing::warn!("HandoffAccepted: entity {} not found in registry", entity_id.0);
        return;
    };

    // Mark as PendingHandoff; ghost updates will follow until HandoffCompleted is sent.
    commands
        .entity(bevy_entity)
        .insert(PendingHandoff { destination });

    tracing::info!("entity {} accepted by shard {} — entering ghost phase", entity_id.0, destination.0);
}

/// Source shard received HandoffRejected → clear pending handoff state.
pub fn handle_handoff_rejected_on_source(
    commands: &mut Commands,
    registry: &SharedEntityRegistry,
    entity_id: EntityId,
) {
    let Some((_, ent_reg)) = registry.try_lock() else {
        tracing::warn!("could not lock registry for HandoffRejected entity {}", entity_id.0);
        return;
    };

    let Some(bevy_entity) = ent_reg.get_bevy_entity(&entity_id) else {
        tracing::warn!("HandoffRejected: entity {} not found in registry", entity_id.0);
        return;
    };

    commands.entity(bevy_entity).remove::<PendingHandoff>();
    tracing::info!("HandoffRejected for entity {} — handoff cancelled", entity_id.0);
}

/// Dest shard received HandoffCompleted → promote ghost to fully authoritative.
pub fn promote_ghost_entities(
    mut commands: Commands,
    mut ev_reader: MessageReader<PromoteGhostEvent>,
    shared_registry: ResMut<SharedEntityRegistry>,
    ghost_query: Query<(Entity, &NetworkEntityId, &Position, &Velocity, &Ghost)>,
    broker: ResMut<BrokerShardPeer>,
    config: Res<ServerConfig>,
) {
    for ev in ev_reader.read() {
        let Some(bevy_entity) = ghost_query
            .iter()
            .find_map(|(e, nid, ..)| (nid.0 == ev.entity_id).then_some(e))
        else {
            tracing::warn!("PromoteGhost: entity {} not found as Ghost", ev.entity_id.0);
            continue;
        };

        commands
            .entity(bevy_entity)
            .remove::<Ghost>()
            .insert(Authoritative);

        tracing::info!("entity {} promoted from Ghost to Authoritative", ev.entity_id.0);
    }
}