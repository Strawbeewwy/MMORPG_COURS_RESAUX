use bevy::prelude::*;

pub const DEFAULT_AREA_OF_INTEREST_RADIUS: f32 = 250.0;

pub fn is_inside_area_of_interest(
    observer_position: Vec2,
    target_position: Vec2,
    radius: f32,
) -> bool {
    distance_squared(observer_position, target_position) <= radius * radius
}

fn distance_squared(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx * dx + dy * dy
}


