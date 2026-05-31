use bevy::prelude::*;
use shared::protocol::broker::{encode_message, BrokerMessage};
use crate::messages::HandoffRequestMsg;
use crate::resources::client_map::{ClientMap, ClientTransferState};
use crate::resources::net_handles::ShardListener;

/// Consume HandoffRequestMsg events and send the wire-level HandoffRequest to
/// the destination shard via the ShardListener.
///
/// Marks the client as `PendingHandoff` to prevent duplicate requests.
/// The state is cleared when the destination shard replies with HandoffAck
/// (handled in `poll_shard_events` → `handle_shard_message`).
pub fn handle_handoff_requests(
    mut ev_reader: MessageReader<HandoffRequestMsg>,
    mut client_map: ResMut<ClientMap>,
    listener: Res<ShardListener>,
) {
    for req in ev_reader.read() {
        // Guard: skip if already transferring.
        if !client_map.is_stable(req.client_id.into()) {
            tracing::debug!(
                "HandoffRequest for client {} dropped — already in PendingHandoff",
                req.client_id.0
            );
            continue;
        }

        let payload = match encode_message(&BrokerMessage::HandoffRequest {
            client_id: req.client_id,
            from_shard: req.from_shard,
            to_shard: req.to_shard,
        }) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    "failed to encode HandoffRequest for client {}: {e}",
                    req.client_id.0
                );
                continue;
            }
        };

        if listener.send_to_shard(req.to_shard.0, payload) {
            client_map.set_state(
                req.client_id.into(),
                ClientTransferState::PendingHandoff {
                    destination_shard: req.to_shard.0,
                },
            );
            tracing::info!(
                "HandoffRequest sent: client {} from shard {} → shard {}",
                req.client_id.0, req.from_shard.0, req.to_shard.0
            );
        } else {
            tracing::warn!(
                "HandoffRequest for client {} dropped — shard {} not reachable",
                req.client_id.0, req.to_shard.0
            );
            // Do not mark as pending — the next CrossingAlert will retry.
        }
    }
}

