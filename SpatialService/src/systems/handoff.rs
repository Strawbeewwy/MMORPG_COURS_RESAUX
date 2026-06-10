use bevy::prelude::*;
use shared::protocol::{encode_message, NetworkMessage};
use crate::messages::HandoffRequestMsg;
use crate::resources::client_map::{ClientMap, ClientTransferState};
use crate::resources::entity_map::{EntityMap, EntityTransferState};
use crate::resources::net_handles::{BrokerClient, ShardListener};

/// Consume HandoffRequestMsg events and send the wire-level HandoffRequest to
/// the destination shard via the ShardListener.
///
/// Marks the client as `PendingHandoff` to prevent duplicate requests.
/// The state is cleared when the destination shard replies with HandoffAck
/// (handled in `poll_shard_events` → `handle_shard_message`).
pub fn handle_handoff_start(
    mut ev_reader: MessageReader<HandoffRequestMsg>,
    entity_map: ResMut<EntityMap>,
    broker: ResMut<BrokerClient>,
) {
    for req in ev_reader.read() {
        // Guard: skip if already transferring.
        if !entity_map.is_stable(req.entity_id.into()) {
            tracing::debug!(
                "HandoffRequest for client {} dropped — already in PendingHandoff",
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
                tracing::error!(
                    "failed to encode HandoffRequest for client {}: {e}",
                    req.entity_id.0
                );
                continue;
            }
        };

        if let Err(error) = broker.handle.send(payload) {
            tracing::error!("failed to send packet to broker: {error:#}");
            return;
        }

        
        tracing::info!(
            "HandoffRequest sent: client {} from shard {} → shard {}",
            req.entity_id.0, req.from_shard.0, req.to_shard.0
        );
        
        // Do not mark as pending — the next CrossingAlert will retry.
    }
}

