
use crate::config::ServerConfig;
use crate::net::input::handle_broker_client_input;
use crate::world::state::{EntityRegistry, handle_register_client};
use bevy::prelude::*;
use shared::protocol::NetVec2;

pub const DEFAULT_AREA_OF_INTEREST_RADIUS: f32 = 250.0;

pub fn is_inside_area_of_interest(
    observer_position: NetVec2,
    target_position: NetVec2,
    radius: f32,
) -> bool {
    distance_squared(observer_position, target_position) <= radius * radius
}

fn distance_squared(a: NetVec2, b: NetVec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    (dx * dx + dy * dy) as f32
}


