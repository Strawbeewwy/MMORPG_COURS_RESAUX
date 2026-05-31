use shared::protocol::{EntityId, NetVec2};

pub const TAG_HANDOFF_REQUEST: u8 = 0x20;
pub const TAG_HANDOFF_ACCEPTED: u8 = 0x21;
pub const TAG_HANDOFF_REJECTED: u8 = 0x22;
pub const TAG_GHOST_UPDATE: u8 = 0x23;
pub const TAG_HANDOFF_COMPLETE: u8 = 0x24;

#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub enum EntityState {
    Owned,
    PendingHandoff,
    Ghost,
}


#[derive(Debug, Clone)]
pub enum InterShardMessage {
    HandoffRequest {
        entity_id: EntityId,
        position: NetVec2,
        velocity: NetVec2,
        entity_state: EntityState
    },
    HandoffAccepted {
        entity_id: EntityId,
    },
    HandoffRejected {
        entity_id: EntityId,
    },
    GhostUpdate {
        entity_id: EntityId,
    },
    HandoffCompleted { // from client to broker
        entity_id: EntityId,
    },
}