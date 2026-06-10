use bevy::prelude::*;
use shared::protocol::{
    ClientId,
    CLIENT_INPUT_LEN,
};

use crate::world::state::SharedEntityRegistry;
use crate::world::Velocity;


pub fn apply_client_input(
    shared_registry: &SharedEntityRegistry,
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
    velocities: &mut Query<&mut Velocity>,
) {

    let movement_x = read_f32_le(&input[0..4]);
    let movement_y = read_f32_le(&input[4..8]);

    if !movement_x.is_finite() || !movement_y.is_finite() {
        warn!(
            "invalid client input: client_id={} movement_x={} movement_y={}",
            client_id.0,
            movement_x,
            movement_y
        );
            return;
    }

    match shared_registry.try_lock() {
        Some((cli_registry, ent_registry))=> {
            let entity_id = cli_registry.client_to_entity.get(&client_id).copied();


            let Some(entity_id) = entity_id else {
                warn!(
                    "received input for unknown client_id={}",
                    client_id.0
                    );
                return;
            };

            let Some(bevy_entity) = ent_registry.get_bevy_entity(&entity_id) else {
                warn!(
                    "no bevy entity for network entity_id={}",
                    entity_id.0
                    );
                return;
            };

            let Ok(mut velocity) = velocities.get_mut(bevy_entity) else {
                warn!(
                    "controlled entity has no Velocity component: entity_id={}",
                    entity_id.0
                    );
                return;
            };

            velocity.0 = Vec2{
                x: movement_x,
                y: movement_y,
            }

        }

        None => {
            warn!("could not lock player registry for client input");
            return;
        }
    };
}


fn read_f32_le(bytes: &[u8]) -> f32 {
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}