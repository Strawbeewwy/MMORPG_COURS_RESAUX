//! Remote entity registry with linear interpolation.
//!
//! `EntityRegistry` is a Godot `Node2D` subclass that:
//!   - spawns/despawns `RemotePlayer` nodes based on `player_joined` / `player_left` signals
//!   - smoothly interpolates remote positions each frame using `lerp`
use godot::classes::INode2D;
use godot::prelude::*;
use std::collections::HashMap;

/// Target state for a remote player — set from the network thread signal.
#[derive(Debug, Clone, Copy)]
struct RemoteState {
    target: Vector2,
    current: Vector2,
}

/// Godot node that manages all remote players.
/// Place it in the scene tree and connect the `NetworkClient` signals to it.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct EntityRegistry {
    base: Base<Node2D>,
    /// Map from server client_id → (scene node, interpolation state)
    entities: HashMap<i64, (Gd<Node2D>, RemoteState)>,
    /// Interpolation speed factor (0 = no lerp, 1 = instant snap).
    #[var]
    lerp_speed: f32,
    /// Scene to instantiate for each remote player.
    #[var]
    remote_player_scene: Option<Gd<PackedScene>>,
}

#[godot_api]
impl INode2D for EntityRegistry {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            entities: HashMap::new(),
            lerp_speed: 10.0,
            remote_player_scene: None,
        }
    }

    fn process(&mut self, delta: f64) {
        let alpha = (self.lerp_speed * delta as f32).clamp(0.0, 1.0);
        for (node, state) in self.entities.values_mut() {
            state.current = state.current.lerp(state.target, alpha);
            node.set_position(state.current);
        }
    }
}

#[godot_api]
impl EntityRegistry {
    /// Called by NetworkClient's `position_received` signal.
    #[func]
    fn on_position_received(&mut self, client_id: i64, x: f32, y: f32) {
        let target = Vector2::new(x, y);
        if let Some((_, state)) = self.entities.get_mut(&client_id) {
            state.target = target;
        }
    }

    /// Called by NetworkClient's `player_joined` signal.
    #[func]
    fn on_player_joined(&mut self, client_id: i64) {
        if self.entities.contains_key(&client_id) {
            return;
        }
        let node: Gd<Node2D> = match &self.remote_player_scene {
            Some(scene) => scene.instantiate_as::<Node2D>(),
            None => Node2D::new_alloc(),
        };
        let start = Vector2::ZERO;
        self.base_mut().add_child(&node.clone().upcast::<Node>());
        self.entities.insert(
            client_id,
            (node, RemoteState { target: start, current: start }),
        );
        tracing::debug!("EntityRegistry: spawned entity for client {client_id}");
    }

    /// Called by NetworkClient's `player_left` signal.
    #[func]
    fn on_player_left(&mut self, client_id: i64) {
        if let Some((mut node, _)) = self.entities.remove(&client_id) {
            node.queue_free();
            tracing::debug!("EntityRegistry: removed entity for client {client_id}");
        }
    }
}
