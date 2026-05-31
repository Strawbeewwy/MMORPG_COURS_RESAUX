use bevy::prelude::*;
use shared::protocol::EntityId;
use shared::protocol::game::EntityType;

#[derive(Debug, Clone)]
pub struct ClientRenderEntity {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub position: Vec2,
    pub color: Color,
    pub size: Vec2,
    pub z_index: f32,
}

#[derive(Component)]
pub struct RenderedEntity {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
}

