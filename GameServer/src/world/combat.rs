/// Combat orchestration for 5SecsSwap.
///
/// Responsibilities:
///   1. Track the global 5-second colour-swap timer.
///   2. Track each player's current `ColorTeam` (assigned at spawn, flipped every swap).
///   3. Process player melee attacks (raycast-style line vs enemy AABB).
///   4. Process player shoot actions (spawn a `ProjectileData`).
///   5. Process player dash actions (velocity burst).
///   6. Collect score updates and publish them via `WorldUpdate`.
///
/// Action flags are read from `PendingActions` which is populated by `input.rs`.
use bevy::prelude::*;
use bevy::platform::collections::HashMap;
use shared::protocol::broker::ClientId;
use shared::protocol::game::combat::{ActionFlags, ColorTeam};
use shared::protocol::game::enemy::EnemyId;
use shared::protocol::NetVec2;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Seconds between global colour swaps.
pub const COLOR_SWAP_INTERVAL: f32 = 5.0;
/// Dash impulse — added on top of current velocity.
pub const DASH_SPEED: f32 = 600.0;
/// Dash duration in seconds (the burst lasts this long server-side).
pub const DASH_DURATION: f32 = 0.15;
/// Cooldown between dashes.
pub const DASH_COOLDOWN: f32 = 1.5;
/// Semi-auto shoot cooldown.
pub const SHOOT_COOLDOWN: f32 = 0.35;
/// Melee cooldown.
pub const MELEE_COOLDOWN: f32 = 0.5;

// ─── Resources ────────────────────────────────────────────────────────────────

/// Global colour-swap state.
#[derive(Debug, Resource)]
pub struct ColorSwapTimer {
    pub elapsed:    f32,
    pub swap_index: u64,
}

impl Default for ColorSwapTimer {
    fn default() -> Self {
        Self { elapsed: 0.0, swap_index: 0 }
    }
}

impl ColorSwapTimer {
    pub fn current_color(&self) -> ColorTeam {
        ColorTeam::from_swap_index(self.swap_index)
    }
}

/// Per-player combat state tracked by the server.
#[derive(Debug, Clone)]
pub struct PlayerCombatState {
    /// The team colour this player currently uses for attacks.
    pub color:          ColorTeam,
    /// Cooldown timers.
    pub dash_cd:        f32,
    pub shoot_cd:       f32,
    pub melee_cd:       f32,
    /// Current dash burst timer (>0 means dashing).
    pub dash_timer:     f32,
    /// Dash direction.
    pub dash_dir:       Vec2,
    /// Score accumulated this session.
    pub score:          u32,
    /// Score sent to client last tick (delta tracking).
    pub score_sent:     u32,
}

impl PlayerCombatState {
    pub fn new(color: ColorTeam) -> Self {
        Self {
            color,
            dash_cd:    0.0,
            shoot_cd:   0.0,
            melee_cd:   0.0,
            dash_timer: 0.0,
            dash_dir:   Vec2::ZERO,
            score:      0,
            score_sent: 0,
        }
    }
}

/// Pending actions decoded from ClientInput byte 8 — written by `input.rs`.
#[derive(Debug, Default, Resource)]
pub struct PendingActions {
    /// client_id → (action_flags_byte, look_direction)
    pub actions: HashMap<ClientId, (u8, Vec2)>,
}

/// Per-player combat states.
#[derive(Debug, Default, Resource)]
pub struct PlayerCombatRegistry {
    pub states: HashMap<ClientId, PlayerCombatState>,
    /// Counter used to assign alternating colours at spawn.
    spawn_counter: u64,
}

impl PlayerCombatRegistry {
    /// Register a player and assign their starting colour.
    pub fn register(&mut self, client_id: ClientId) {
        let color = ColorTeam::from_spawn_counter(self.spawn_counter);
        self.spawn_counter += 1;
        self.states.insert(client_id, PlayerCombatState::new(color));
    }

    pub fn remove(&mut self, client_id: &ClientId) {
        self.states.remove(client_id);
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Tick the global swap timer; emit `ColorSwap` via `PendingSwapEvents`.
pub fn color_swap_system(
    time: Res<Time>,
    mut timer: ResMut<ColorSwapTimer>,
    mut pending_events: ResMut<PendingSwapEvents>,
) {
    timer.elapsed += time.delta_secs();
    if timer.elapsed >= COLOR_SWAP_INTERVAL {
        timer.elapsed -= COLOR_SWAP_INTERVAL;
        timer.swap_index += 1;
        pending_events.0.push(timer.swap_index);
        tracing::info!(
            "ColorSwap! swap_index={} active_color={:?}",
            timer.swap_index,
            timer.current_color()
        );
    }
}

/// Staging buffer for colour-swap events (read by `publish_gameplay_updates`).
#[derive(Debug, Default, Resource)]
pub struct PendingSwapEvents(pub Vec<u64>);

/// Tick cooldowns and process pending player actions.
pub fn player_combat_system(
    time: Res<Time>,
    mut pending: ResMut<PendingActions>,
    mut combat_reg: ResMut<PlayerCombatRegistry>,
    mut proj_reg: ResMut<crate::world::projectile::ProjectileRegistry>,
    mut enemy_reg: ResMut<crate::world::enemy::EnemyRegistry>,
    player_reg: Res<crate::net::network_event::SharedPlayerRegistry>,
) {
    use crate::world::enemy::{MELEE_RANGE, MELEE_HALF_ANGLE, PROJECTILE_HIT_RADIUS};

    let dt = time.delta_secs();

    // Snapshot player positions for melee/shoot origin.
    let player_positions: HashMap<ClientId, Vec2> = {
        let Ok(reg) = player_reg.inner.try_lock() else { return };
        reg.player_client
            .iter()
            .filter_map(|(pid, cid)| {
                reg.players.get(pid).map(|p| {
                    let (x, y) = p.position.to_f32();
                    (*cid, Vec2::new(x, y))
                })
            })
            .collect()
    };

    let actions: Vec<(ClientId, u8, Vec2)> = pending
        .actions
        .drain()
        .map(|(cid, (flags, look))| (cid, flags, look))
        .collect();

    for (client_id, flags_byte, look_dir) in actions {
        let flags = ActionFlags(flags_byte);
        let Some(state) = combat_reg.states.get_mut(&client_id) else { continue };

        // Tick cooldowns.
        state.dash_cd   = (state.dash_cd   - dt).max(0.0);
        state.shoot_cd  = (state.shoot_cd  - dt).max(0.0);
        state.melee_cd  = (state.melee_cd  - dt).max(0.0);
        state.dash_timer = (state.dash_timer - dt).max(0.0);

        let origin = player_positions.get(&client_id).copied().unwrap_or(Vec2::ZERO);

        // Dash.
        if flags.dash() && state.dash_cd <= 0.0 {
            let dir = if look_dir.length_squared() > 0.001 {
                look_dir.normalize()
            } else {
                Vec2::Y
            };
            state.dash_dir   = dir;
            state.dash_timer = DASH_DURATION;
            state.dash_cd    = DASH_COOLDOWN;
            // Apply dash impulse to player velocity via player registry.
            let dash_vel = dir * DASH_SPEED;
            if let Ok(mut reg) = player_reg.inner.try_lock() {
                if let Some(&pid) = reg.client_player.get(&client_id) {
                    if let Some(player) = reg.players.get_mut(&pid) {
                        player.velocity = NetVec2::from_f32(
                            dash_vel.x, dash_vel.y, NetVec2::DEFAULT_PRECISION,
                        );
                    }
                }
            }
        }

        // Shoot.
        if flags.shoot() && state.shoot_cd <= 0.0 {
            state.shoot_cd = SHOOT_COOLDOWN;
            let dir = if look_dir.length_squared() > 0.001 {
                look_dir.normalize()
            } else {
                Vec2::NEG_Y
            };
            proj_reg.spawn(client_id, origin, dir, state.color);
        }

        // Melee — raycast: check every enemy inside range + cone.
        if flags.melee() && state.melee_cd <= 0.0 {
            state.melee_cd = MELEE_COOLDOWN;
            let forward = if look_dir.length_squared() > 0.001 {
                look_dir.normalize()
            } else {
                Vec2::NEG_Y
            };

            let enemy_ids: Vec<EnemyId> = enemy_reg.enemies.keys().copied().collect();
            for eid in enemy_ids {
                let Some(enemy) = enemy_reg.enemies.get(&eid) else { continue };
                if enemy.color != state.color { continue }

                let to_enemy = enemy.position - origin;
                let dist     = to_enemy.length();
                if dist > MELEE_RANGE { continue }

                let angle = forward.angle_to(to_enemy.normalize_or_zero()).abs();
                if angle <= MELEE_HALF_ANGLE {
                    let killer = Some(client_id);
                    let killed = enemy_reg.damage(eid, 2, killer);
                    if killed {
                        if let Some(cs) = combat_reg.states.get_mut(&client_id) {
                            cs.score += 10;
                        }
                    }
                }
            }
        }
    }
}

/// Accumulate kill credits from projectile collisions into player scores.
pub fn score_collection_system(
    mut enemy_reg: ResMut<crate::world::enemy::EnemyRegistry>,
    mut combat_reg: ResMut<PlayerCombatRegistry>,
) {
    let died = std::mem::take(&mut enemy_reg.died_this_tick);
    for (_eid, killer) in died {
        if let Some(cid) = killer {
            if let Some(state) = combat_reg.states.get_mut(&cid) {
                state.score += 5;
            }
        }
    }
}

