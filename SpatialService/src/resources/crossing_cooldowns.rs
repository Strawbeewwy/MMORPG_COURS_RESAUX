use bevy::prelude::*;
use std::collections::HashMap;

/// Deduplicates CrossingAlert emissions.
/// A (client_id, sorted shard pair) combo is suppressed for `cooldown_ticks` ticks
/// after being emitted, preventing a flood when a client lingers on a boundary.
#[derive(Resource)]
pub struct CrossingCooldowns {
    /// Maps (client_id, canonical shard pair) → remaining cooldown ticks.
    active: HashMap<(u32, (u32, u32)), u32>,
    /// Number of ticks to suppress a repeated alert for the same pair.
    pub cooldown_ticks: u32,
}

impl Default for CrossingCooldowns {
    fn default() -> Self {
        Self {
            active: HashMap::new(),
            cooldown_ticks: 10, // ~0.5 s at 20 Hz
        }
    }
}

impl CrossingCooldowns {
    /// Returns true and starts/resets the cooldown if the alert should be emitted.
    /// Returns false if it is still on cooldown.
    pub fn should_emit(&mut self, client_id: u32, shard_a: u32, shard_b: u32) -> bool {
        let key = (client_id, canonical_pair(shard_a, shard_b));
        if self.active.contains_key(&key) {
            false
        } else {
            self.active.insert(key, self.cooldown_ticks);
            true
        }
    }

    /// Tick down all cooldowns and remove expired entries.
    pub fn tick(&mut self) {
        self.active.retain(|_, remaining| {
            *remaining = remaining.saturating_sub(1);
            *remaining > 0
        });
    }
}

/// Normalise a shard pair so (a,b) == (b,a).
#[inline]
fn canonical_pair(a: u32, b: u32) -> (u32, u32) {
    if a <= b { (a, b) } else { (b, a) }
}

