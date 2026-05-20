use crate::render::renderable::RenderedEntity;
use crate::world::state::LocalWorldState;
use bevy::prelude::*;

const WORLD_TO_SCREEN_SCALE: f32 = 32.0;

pub fn render_entities(
    mut commands: Commands,
    world_state: Res<LocalWorldState>,
    mut rendered_entities: Query<(Entity, &RenderedEntity, &mut Transform, &mut Sprite)>,
) {
    for render_entity in world_state.render_entities.values() {
        let position = Vec3::new(
            render_entity.position.x * WORLD_TO_SCREEN_SCALE,
            render_entity.position.y * WORLD_TO_SCREEN_SCALE,
            render_entity.z_index,
        );

        let mut entity_already_exists = false;

        for (_, rendered_entity, mut transform, mut sprite) in rendered_entities.iter_mut() {
            if rendered_entity.entity_id == render_entity.entity_id
                && rendered_entity.entity_type == render_entity.entity_type
            {
                transform.translation = position;
                sprite.color = render_entity.color;
                sprite.custom_size = Some(render_entity.size);
                entity_already_exists = true;
                break;
            }
        }

        if entity_already_exists {
            continue;
        }

        commands.spawn((
            Sprite {
                color: render_entity.color,
                custom_size: Some(render_entity.size),
                ..default()
            },
            Transform::from_translation(position),
            RenderedEntity {
                entity_id: render_entity.entity_id.clone(),
                entity_type: render_entity.entity_type,
            },
        ));
    }

    for (entity, rendered_entity, _, _) in rendered_entities.iter_mut() {
        if !world_state
            .render_entities
            .contains_key(&rendered_entity.entity_id)
        {
            commands.entity(entity).despawn();
        }
    }
}