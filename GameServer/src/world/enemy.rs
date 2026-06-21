/// Enemy simulation for 5SecsSwap.
///
/// Enemies spawn in waves, chase the nearest player, and die when hit by an
/// attack whose `ColorTeam` matches the enemy's own colour.
///
/// The registry is a plain Bevy `Resource` — all mutation happens on the main
/// Bevy thread so no extra Mutex is needed.
use bevy::prelude::*;
use bevy::platform::collections::HashMap;
use shared::ClientId;
use shared::protocol::game::combat::ColorTeam;
use shared::protocol::game::enemy::{EnemyId, EnemySnapshot};
use shared::protocol::NetVec2;
pub use world::SharedEntityRegistry;
use crate::world;
// ─── Constants ────────────────────────────────────────────────────────────────

/// Pixels (world units) per second enemies move toward their target.
const ENEMY_SPEED: f32 = 60.0;
/// Base HP for a standard enemy.
const ENEMY_BASE_HP: u8 = 3;
/// How many enemies to target on the map.
const ENEMY_SPAWN_TARGET: usize = 200;
/// Spawn rate: enemies per second (server may spawn multiple at once to reach target).
const ENEMY_SPAWN_RATE: f32 = 10.0;
/// Spawn radius around origin.
const SPAWN_RADIUS: f32 = 1800.0;
/// Melee attack range (world units).
pub const MELEE_RANGE: f32 = 80.0;
/// Melee cone half-angle in radians (~60°).
pub const MELEE_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_3;
/// Projectile collision radius.
pub const PROJECTILE_HIT_RADIUS: f32 = 16.0;

// ─── Data ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EnemyData {
    pub id:       EnemyId,
    pub position: Vec2,
    pub color:    ColorTeam,
    pub hp:       u8,
    /// Cached move direction toward current target (updated each tick).
    pub velocity: Vec2,
}

impl EnemyData {
    pub fn snapshot(&self) -> EnemySnapshot {
        EnemySnapshot {
            id:       self.id,
            position: NetVec2::from_f32(self.position.x, self.position.y, NetVec2::DEFAULT_PRECISION),
            color:    self.color,
            hp:       self.hp,
        }
    }
}

// ─── Registry resource ────────────────────────────────────────────────────────

#[derive(Debug, Default, Resource)]
pub struct EnemyRegistry {
    pub enemies:       HashMap<EnemyId, EnemyData>,
    next_id:           EnemyId,
    spawn_counter:     u64,
    spawn_accumulator: f32,
    /// Enemies that died this tick — published once then cleared.
    pub died_this_tick: Vec<(EnemyId, Option<ClientId>)>,
}

impl EnemyRegistry {
    fn next_id(&mut self) -> EnemyId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Spawn one enemy at a fixed ring position around the origin.
    pub fn spawn_one(&mut self) {
        let id     = self.next_id();
        let angle  = (id as f32) * 2.399_631 /* golden angle rad */;
        let radius = SPAWN_RADIUS * 0.8 + (id as f32 % 400.0);
        let pos    = Vec2::new(angle.cos() * radius, angle.sin() * radius);
        let color  = ColorTeam::from_spawn_counter(self.spawn_counter);
        self.spawn_counter += 1;

        self.enemies.insert(id, EnemyData {
            id,
            position: pos,
            color,
            hp:       ENEMY_BASE_HP,
            velocity: Vec2::ZERO,
        });
    }

    /// Apply `delta` damage to enemy, return true if it died.
    pub fn damage(
        &mut self,
        enemy_id: EnemyId,
        dmg: u8,
        killer: Option<ClientId>,
    ) -> bool {
        if let Some(e) = self.enemies.get_mut(&enemy_id) {
            if e.hp <= dmg {
                e.hp = 0;
                let id = e.id;
                self.enemies.remove(&id);
                self.died_this_tick.push((id, killer));
                return true;
            }
            e.hp -= dmg;
        }
        false
    }

    pub fn snapshots(&self) -> Vec<EnemySnapshot> {
        self.enemies.values().map(|e| e.snapshot()).collect()
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Spawn enemies until we reach `ENEMY_SPAWN_TARGET`.
pub fn enemy_spawn_system(
    time: Res<Time>,
    mut registry: ResMut<EnemyRegistry>,
) {
    if registry.enemies.len() >= ENEMY_SPAWN_TARGET {
        return;
    }
    registry.spawn_accumulator += ENEMY_SPAWN_RATE * time.delta_secs();
    let to_spawn = registry.spawn_accumulator as usize;
    registry.spawn_accumulator -= to_spawn as f32;

    let missing = ENEMY_SPAWN_TARGET.saturating_sub(registry.enemies.len());
    let count   = to_spawn.min(missing);
    for _ in 0..count {
        registry.spawn_one();
    }
}

/// Move all enemies toward the nearest player each tick.
pub fn enemy_ai_system(
    time: Res<Time>,
    mut enemies: ResMut<EnemyRegistry>,
    player_reg: Res<SharedEntityRegistry>,
) {
    // Snapshot player positions without holding the lock during movement.
    // let player_positions: Vec<Vec2> = {
    //     let Ok(reg) = player_reg.inner.try_lock() else { return };
    //     reg.players.values().map(|p| {
    //         let (x, y) = p.position.to_f32();
    //         Vec2::new(x, y)
    //     }).collect()
    // };
    // 
    // if player_positions.is_empty() { return; }
    // 
    // let dt   = time.delta_secs();
    // let speed = ENEMY_SPEED * dt;
    // 
    // for enemy in enemies.enemies.values_mut() {
    //     // Find the nearest player.
    //     let nearest = player_positions.iter().copied()
    //         .min_by(|a, b| {
    //             let da = a.distance_squared(enemy.position);
    //             let db = b.distance_squared(enemy.position);
    //             da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    //         });
    // 
    //     if let Some(target) = nearest {
    //         let dir = (target - enemy.position).normalize_or_zero();
    //         enemy.velocity = dir;
    //         enemy.position += dir * speed;
    //     }
    // }
}

