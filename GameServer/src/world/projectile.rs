/// Projectile simulation for 5SecsSwap.
///
/// Projectiles are spawned by `combat.rs` when a player fires and are moved
/// here each tick.  Collision against enemies is also resolved here.
use bevy::prelude::*;
use bevy::platform::collections::HashMap;
use shared::protocol::broker::ClientId;
use shared::protocol::game::combat::ColorTeam;
use shared::protocol::game::projectile::{ProjectileId, ProjectileSnapshot};
use shared::protocol::NetVec2;

/// Projectile travel speed in world units per second.
const PROJECTILE_SPEED: f32 = 400.0;
/// Maximum lifetime in seconds before automatic despawn.
const PROJECTILE_TTL: f32 = 3.0;

// ─── Data ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProjectileData {
    pub id:              ProjectileId,
    pub owner_client_id: ClientId,
    pub position:        Vec2,
    pub direction:       Vec2,
    pub color:           ColorTeam,
    pub alive:           bool,
    pub ttl:             f32,
}

impl ProjectileData {
    pub fn snapshot(&self) -> ProjectileSnapshot {
        ProjectileSnapshot {
            id:              self.id,
            owner_client_id: self.owner_client_id,
            position:        NetVec2::from_f32(self.position.x, self.position.y, NetVec2::DEFAULT_PRECISION),
            direction:       NetVec2::from_f32(self.direction.x, self.direction.y, NetVec2::DEFAULT_PRECISION),
            color:           self.color,
            alive:           self.alive,
        }
    }
}

// ─── Registry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Resource)]
pub struct ProjectileRegistry {
    pub projectiles: HashMap<ProjectileId, ProjectileData>,
    next_id:         ProjectileId,
}

impl ProjectileRegistry {
    pub fn spawn(
        &mut self,
        owner: ClientId,
        position: Vec2,
        direction: Vec2,
        color: ColorTeam,
    ) -> ProjectileId {
        let id = self.next_id;
        self.next_id += 1;
        self.projectiles.insert(id, ProjectileData {
            id,
            owner_client_id: owner,
            position,
            direction: direction.normalize_or_zero(),
            color,
            alive: true,
            ttl: PROJECTILE_TTL,
        });
        id
    }

    pub fn snapshots(&self) -> Vec<ProjectileSnapshot> {
        self.projectiles.values().map(|p| p.snapshot()).collect()
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Move projectiles and remove expired ones.
pub fn projectile_movement_system(
    time: Res<Time>,
    mut proj_reg: ResMut<ProjectileRegistry>,
) {
    let dt    = time.delta_secs();
    let speed = PROJECTILE_SPEED * dt;

    let ids: Vec<ProjectileId> = proj_reg.projectiles.keys().copied().collect();
    for id in ids {
        if let Some(p) = proj_reg.projectiles.get_mut(&id) {
            p.position += p.direction * speed;
            p.ttl      -= dt;
            if p.ttl <= 0.0 {
                p.alive = false;
            }
        }
    }

    // Remove dead projectiles.
    proj_reg.projectiles.retain(|_, p| p.alive);
}

/// Check projectile-vs-enemy collisions.
/// A hit occurs when the projectile's colour matches the enemy's colour.
pub fn projectile_collision_system(
    mut proj_reg: ResMut<ProjectileRegistry>,
    mut enemy_reg: ResMut<crate::world::enemy::EnemyRegistry>,
) {
    use crate::world::enemy::PROJECTILE_HIT_RADIUS;

    let proj_ids: Vec<ProjectileId> = proj_reg.projectiles.keys().copied().collect();

    for pid in proj_ids {
        let Some(proj) = proj_reg.projectiles.get(&pid) else { continue };
        if !proj.alive { continue }

        // Check against every enemy.
        let enemy_ids: Vec<u32> = enemy_reg.enemies.keys().copied().collect();
        for eid in enemy_ids {
            let Some(enemy) = enemy_reg.enemies.get(&eid) else { continue };

            // Colour gating: projectile only hits enemies of the same colour.
            if proj.color != enemy.color { continue }

            let dist = proj.position.distance(enemy.position);
            if dist < PROJECTILE_HIT_RADIUS {
                let owner = proj.owner_client_id;
                enemy_reg.damage(eid, 1, Some(owner));
                // Destroy projectile on hit.
                if let Some(p) = proj_reg.projectiles.get_mut(&pid) {
                    p.alive = false;
                }
                break;
            }
        }
    }

    // Remove dead projectiles from collisions.
    proj_reg.projectiles.retain(|_, p| p.alive);
}

