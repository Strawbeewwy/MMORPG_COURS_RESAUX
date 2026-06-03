//! UI bindings — Godot Node subclasses for HUD elements.
//!
//! `ConnectionStatusLabel` updates a Godot Label to show the current
//! network connection status (Connecting / Connected / Disconnected).
use godot::prelude::*;

/// Attach this script to a `Label` node in your HUD scene.
/// It polls the NetworkClient autoload and updates its text each frame.
#[derive(GodotClass)]
#[class(base=Label)]
pub struct ConnectionStatusLabel {
    base: Base<Label>,
}

#[godot_api]
impl ILabel for ConnectionStatusLabel {
    fn init(base: Base<Label>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        self.base_mut().set_text("Connecting…".into());
    }
}

#[godot_api]
impl ConnectionStatusLabel {
    /// Call this from the NetworkClient `connected` signal (add it later).
    #[func]
    fn on_connected(&mut self) {
        self.base_mut().set_text("Connected".into());
    }

    /// Call this from the NetworkClient `disconnected` signal.
    #[func]
    fn on_disconnected(&mut self) {
        self.base_mut().set_text("Disconnected — reconnecting…".into());
    }
}

/// Debug overlay — draws shard boundaries in the editor and at runtime.
/// Attach this `@tool`-equivalent Rust node to visualise the QuadTree grid.
#[derive(GodotClass)]
#[class(tool, base=Node2D)]
pub struct ShardBoundaryDebug {
    base: Base<Node2D>,
    /// Size of each shard cell in world units.
    #[var]
    shard_size: Vector2,
    /// Number of shard columns × rows.
    #[var]
    grid_count: Vector2i,
    /// Colour of the boundary lines.
    #[var]
    line_color: Color,
}

#[godot_api]
impl INode2D for ShardBoundaryDebug {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            shard_size: Vector2::new(1024.0, 1024.0),
            grid_count: Vector2i::new(4, 4),
            line_color: Color::from_rgb(0.0, 1.0, 1.0),
        }
    }

    fn draw(&mut self) {
        let size = self.shard_size;
        let cols = self.grid_count.x;
        let rows = self.grid_count.y;
        let color = self.line_color;

        for x in 0..cols {
            for y in 0..rows {
                let origin = Vector2::new(x as f32 * size.x, y as f32 * size.y);
                let rect = Rect2::new(origin, size);
                self.base_mut().draw_rect(rect, color, false, 2.0);
            }
        }
    }
}

