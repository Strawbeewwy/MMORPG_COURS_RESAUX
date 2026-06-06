//! UI bindings — Godot Node subclasses for HUD elements.
//!
//! `ConnectionStatusLabel` updates a Godot Label to show the current
//! network connection status (Connecting / Connected / Disconnected).
use godot::classes::{INode2D, Node2D};
use godot::prelude::*;

// ─── ConnectionStatusLabel ────────────────────────────────────────────────────
// Note: this is a plain Rust struct — status updates are driven from GDScript
// debug_hud.gd which has direct access to the Label node.
// The Rust side exposes no Godot class for this — it is pure GDScript.

// ─── ShardBoundaryDebug ───────────────────────────────────────────────────────

/// Godot Node2D that draws the QuadTree shard grid.
/// Works as a @tool node — visible directly in the Godot editor.
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
                self.base_mut().draw_rect(rect, color);
            }
        }
    }
}
