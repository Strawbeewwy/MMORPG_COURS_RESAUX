use bevy::prelude::*;
use shared::protocol::{encode_message, NetworkMessage};
use crate::messages::HandoffRequestMsg;
use crate::resources::entity_map::{EntityMap, EntityTransferState};
use crate::resources::net_handles::BrokerClient;

/// Send HandoffStart to the broker and mark entity as PendingHandoff.
pub fn handle_handoff_start(
    mut ev_reader: MessageReader<HandoffRequestMsg>,
    mut entity_map: ResMut<EntityMap>,
    broker: ResMut<BrokerClient>,
) {
    for req in ev_reader.read() {
        if !entity_map.is_stable(req.entity_id) {
            tracing::debug!(
                "HandoffStart for entity {} dropped — already PendingHandoff",
                req.entity_id.0
            );
            continue;
        }

        let payload = match encode_message(&NetworkMessage::HandoffStart {
            entity_id: req.entity_id,
            source: req.from_shard,
            destination: req.to_shard,
        }) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("failed to encode HandoffStart for entity {}: {e}", req.entity_id.0);
                continue;
            }
        };

        if let Err(e) = broker.handle.send(payload) {
            tracing::error!("failed to send HandoffStart to broker: {e:#}");
            continue;
        }

        entity_map.set_state(
            req.entity_id,
            EntityTransferState::PendingHandoff { destination_shard: req.to_shard.0 },
        );

        tracing::info!(
            "HandoffStart sent: entity {} from shard {} → shard {}",
            req.entity_id.0, req.from_shard.0, req.to_shard.0
        );
    }
}

