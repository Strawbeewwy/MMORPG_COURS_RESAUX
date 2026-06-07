use bevy::prelude::*;
use shared::protocol::{
    ClientId,
    CLIENT_INPUT_LEN,
};

use crate::world::{
    ClientEntityRegistry,
    EntityRegistry,
    Velocity,
};

#[derive(Message, Debug, Clone, Copy)]
pub struct ClientInputEvent {
    pub client_id: ClientId,
    pub input: [u8; CLIENT_INPUT_LEN],
}

pub fn apply_client_input_events(
    mut message: MessageReader<ClientInputEvent>,
    client_index: Res<ClientEntityRegistry>,
    entity_index: Res<EntityRegistry>,
    mut velocities: Query<&mut Velocity>,
) {
    for event in message.read() {
        let movement_x = read_f32_le(&event.input[0..4]);
        let movement_y = read_f32_le(&event.input[4..8]);

        if !movement_x.is_finite() || !movement_y.is_finite() {
            warn!(
                "invalid client input: client_id={} movement_x={} movement_y={}",
                event.client_id.0,
                movement_x,
                movement_y
            );
            continue;
        }

        let Some(entity_id) = client_index.client_to_entity.get(&event.client_id) else {
            warn!(
                "no controlled entity for client_id={}",
                event.client_id.0
            );
            continue;
        };

        let Some(bevy_entity) = entity_index.get_bevy_entity(entity_id) else {
            warn!(
                "no bevy entity for network entity_id={}",
                entity_id.0
            );
            continue;
        };

        let Ok(mut velocity) = velocities.get_mut(bevy_entity) else {
            warn!(
                "controlled entity has no Velocity component: entity_id={}",
                entity_id.0
            );
            continue;
        };

        velocity.0 = Vec2{
            x: movement_x,
            y: movement_y,
        }
   
    }
}

fn read_f32_le(bytes: &[u8]) -> f32 {
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}