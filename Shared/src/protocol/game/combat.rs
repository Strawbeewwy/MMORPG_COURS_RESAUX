/// Combat protocol types shared between GameServer and all clients.
///
/// The core mechanic of 5SecsSwap:
///   - A global `swap_index` (u64) increments every 5 seconds on the server.
///   - `ColorTeam::from_swap_index(idx)` gives the CURRENT global active color.
///   - Each player is assigned a `ColorTeam` at spawn (random 50/50).
///   - A player attacks only enemies whose `ColorTeam` matches their own.
use serde::{Deserialize, Serialize};
use crate::ClientId;
// ─── Color team ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ColorTeam {
    Red  = 0,
    Blue = 1,
}

impl ColorTeam {
    /// Derive the global active color from the server-wide swap counter.
    /// swap_index 0 → Red, 1 → Blue, 2 → Red, …
    pub fn from_swap_index(idx: u64) -> Self {
        if idx % 2 == 0 { Self::Red } else { Self::Blue }
    }

    /// Flip to the opposite team.
    pub fn opposite(self) -> Self {
        match self {
            Self::Red  => Self::Blue,
            Self::Blue => Self::Red,
        }
    }

    /// Assign a team from a raw counter (used for player spawn: alternates 50/50).
    pub fn from_spawn_counter(n: u64) -> Self {
        if n % 2 == 0 { Self::Red } else { Self::Blue }
    }
}

// ─── Attack type ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackType {
    Projectile,
    Melee,
}

// ─── Action flags encoded in ClientInput byte 8 ───────────────────────────────

/// Bitmask encoding in ClientInput payload[8]:
///   bit 0 — dash
///   bit 1 — melee attack
///   bit 2 — shoot projectile
pub struct ActionFlags(pub u8);

impl ActionFlags {
    pub const DASH:  u8 = 1 << 0;
    pub const MELEE: u8 = 1 << 1;
    pub const SHOOT: u8 = 1 << 2;

    pub fn dash(self)  -> bool { self.0 & Self::DASH  != 0 }
    pub fn melee(self) -> bool { self.0 & Self::MELEE != 0 }
    pub fn shoot(self) -> bool { self.0 & Self::SHOOT != 0 }
}

// ─── Score event ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillCredit {
    pub killer_client_id: Option<ClientId>,
    pub points: u32,
}

