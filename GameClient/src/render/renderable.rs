use bevy::prelude::*;
use shared::protocol::NetVec2;

#[derive(Debug, Clone)]
pub struct ClientRenderEntity {
    pub entity_id: String,
    pub entity_type: RenderedEntityType,
    pub position: NetVec2,
    pub color: Color,
    pub size: Vec2,
    pub z_index: f32,
}

#[derive(Component)]
pub struct RenderedEntity {
    pub entity_id: String,
    pub entity_type: RenderedEntityType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedEntityType {
    Player,
    Enemy,
    Npc,
    Item,
    Projectile,
    Effect,
}