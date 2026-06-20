use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::Resource;
use crate::messages::HandoffRequestMsg;
use shared::protocol::{EntityId, ShardId};

/// we needed a ressouce to keep track of handoffs
/// when we split a quadtree node.
/// since the quad tree creates the shard id but doesn't know
/// when the shard connects to the network we need to keep track
/// of which entity goes where.

#[derive(Resource, Default)]
pub struct PendingHandoffs {
    connected_shards: HashSet<ShardId>,
    pending_by_destination: HashMap<ShardId, Vec<HandoffRequestMsg>>,
}

impl PendingHandoffs {
    pub fn mark_connected(&mut self, shard_id: ShardId) -> Vec<HandoffRequestMsg> {
        self.connected_shards.insert(shard_id);
        self.pending_by_destination
            .remove(&shard_id)
            .unwrap_or_default()
    }

    pub fn mark_disconnected(&mut self, shard_id: ShardId) {
        self.connected_shards.remove(&shard_id);
        self.pending_by_destination.remove(&shard_id);
    }

    pub fn mark_all_disconnected(&mut self) {
        self.connected_shards.clear();
        self.pending_by_destination.clear();
    }

    pub fn is_connected(&self, shard_id: ShardId) -> bool {
        self.connected_shards.contains(&shard_id)
    }

    pub fn queue_or_ready(&mut self, handoff: HandoffRequestMsg) -> Option<HandoffRequestMsg> {
        if self.is_connected(handoff.to_shard) {
            Some(handoff)
        } else {
            self.pending_by_destination
                .entry(handoff.to_shard)
                .or_default()
                .push(handoff);

            None
        }
    }

    pub fn remove_entity(&mut self, entity_id: EntityId) {
        for pending in self.pending_by_destination.values_mut() {
            pending.retain(|handoff| handoff.entity_id != entity_id);
        }

        self.pending_by_destination
            .retain(|_, pending| !pending.is_empty());
    }

    pub fn pending_count_for(&self, shard_id: ShardId) -> usize {
        self.pending_by_destination
            .get(&shard_id)
            .map(Vec::len)
            .unwrap_or(0)
    }
}