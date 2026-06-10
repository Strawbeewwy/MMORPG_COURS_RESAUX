/// Pure Quad Tree structure — no Bevy dependency, fully unit-testable.
///
/// Each internal node subdivides its bounds into 4 equal quadrants (NW, NE, SW, SE).
/// Leaf nodes carry a shard_id that maps to a utils topic ("shard:N").
use bevy::prelude::Resource;
use shared::protocol::ShardId;

/// Axis-aligned bounding rectangle in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Rect {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    /// World square centred on origin.
    pub fn world(half_size: f32) -> Self {
        Self::new(-half_size, -half_size, half_size, half_size)
    }

    #[inline]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x < self.max_x && y >= self.min_y && y < self.max_y
    }

    /// True when the circle (cx, cy, r) intersects this rect (AABB vs circle test).
    pub fn intersects_circle(&self, cx: f32, cy: f32, r: f32) -> bool {
        let nearest_x = cx.clamp(self.min_x, self.max_x);
        let nearest_y = cy.clamp(self.min_y, self.max_y);
        let dx = cx - nearest_x;
        let dy = cy - nearest_y;
        dx * dx + dy * dy <= r * r
    }

    fn mid_x(&self) -> f32 { (self.min_x + self.max_x) * 0.5 }
    fn mid_y(&self) -> f32 { (self.min_y + self.max_y) * 0.5 }

    /// NW / NE / SW / SE quadrants.
    fn quadrants(&self) -> [Rect; 4] {
        let mx = self.mid_x();
        let my = self.mid_y();
        [
            Rect::new(self.min_x, my,       mx,       self.max_y), // NW
            Rect::new(mx,         my,       self.max_x, self.max_y), // NE
            Rect::new(self.min_x, self.min_y, mx,       my),         // SW
            Rect::new(mx,         self.min_y, self.max_x, my),       // SE
        ]
    }
}

#[derive(Resource)]
pub struct QuadTree {
    pub bounds: Rect,
    pub depth: u8,
    pub max_depth: u8,
    pub children: Option<Box<[QuadTree; 4]>>,
    /// Only set on leaf nodes.
    pub shard_id: Option<ShardId>,
}

impl QuadTree {
    /// Build a fully subdivided tree up to `max_depth`.
    /// Leaf shard ids are assigned left-to-right, top-to-bottom (Morton-like order).
    pub fn new(bounds: Rect, max_depth: u8) -> Self {
        let mut counter = 0u32;
        Self::build(bounds, 0, max_depth, &mut counter)
    }

    fn build(bounds: Rect, depth: u8, max_depth: u8, counter: &mut u32) -> Self {
        if depth >= max_depth {
            let shard_id = *counter;
            *counter += 1;
            return Self { bounds, depth, max_depth, children: None, shard_id: Some(ShardId(shard_id)) };
        }

        let quads = bounds.quadrants();
        let children = Box::new([
            Self::build(quads[0], depth + 1, max_depth, counter),
            Self::build(quads[1], depth + 1, max_depth, counter),
            Self::build(quads[2], depth + 1, max_depth, counter),
            Self::build(quads[3], depth + 1, max_depth, counter),
        ]);

        Self { bounds, depth, max_depth, children: Some(children), shard_id: None }
    }

    /// Return the shard_id of the leaf containing `(x, y)`.
    pub fn shard_for(&self, x: f32, y: f32) -> Option<ShardId> {
        if !self.bounds.contains(x, y) {
            return None;
        }
        match &self.children {
            None => self.shard_id,
            Some(children) => children.iter().find_map(|c| c.shard_for(x, y)),
        }
    }

    /// Collect all distinct shard_ids whose leaf bounds intersect a circle of radius `margin`
    /// centred on `(x, y)`. Used to detect proximity to a shard boundary.
    pub fn shards_near(&self, x: f32, y: f32, margin: f32) -> Vec<ShardId> {
        let mut result = Vec::new();
        self.collect_shards_near(x, y, margin, &mut result);
        result.sort_unstable();
        result.dedup();
        result
    }

    /// Zero-allocation variant — caller provides a reusable buffer (cleared on entry).
    /// Prefer this in hot paths (called once per client per tick).
    pub fn shards_near_into(&self, x: f32, y: f32, margin: f32, out: &mut Vec<ShardId>) {
        out.clear();
        self.collect_shards_near(x, y, margin, out);
        out.sort_unstable();
        out.dedup();
    }

    fn collect_shards_near(&self, x: f32, y: f32, margin: f32, out: &mut Vec<ShardId>) {
        if !self.bounds.intersects_circle(x, y, margin) {
            return;
        }
        match &self.children {
            None => {
                if let Some(id) = self.shard_id {
                    out.push(id);
                }
            }
            Some(children) => {
                for child in children.iter() {
                    child.collect_shards_near(x, y, margin, out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> QuadTree {
        // depth=1 → 4 leaves: NW=0, NE=1, SW=2, SE=3
        QuadTree::new(Rect::world(100.0), 1)
    }

    #[test]
    fn shard_for_quadrants() {
        let qt = tree();
        // NW (x<0, y>0)
        assert_eq!(qt.shard_for(-50.0, 50.0), Some(ShardId(0)));
        // NE (x>0, y>0)
        assert_eq!(qt.shard_for(50.0, 50.0), Some(ShardId(1)));
        // SW (x<0, y<0)
        assert_eq!(qt.shard_for(-50.0, -50.0), Some(ShardId(2)));
        // SE (x>0, y<0)
        assert_eq!(qt.shard_for(50.0, -50.0), Some(ShardId(3)));
        // Outside world
        assert_eq!(qt.shard_for(200.0, 0.0), None);
    }

    #[test]
    fn shards_near_boundary() {
        let qt = tree();
        // Point exactly on the X=0 axis with margin covering both sides → 2 shards
        let near = qt.shards_near(0.0, 50.0, 10.0);
        assert!(near.len() >= 2, "expected multiple shards near boundary, got {:?}", near);
    }

    #[test]
    fn shards_near_interior() {
        let qt = tree();
        // Deep inside NW — only one shard should be returned
        let near = qt.shards_near(-80.0, 80.0, 5.0);
        assert_eq!(near, vec![ShardId(0)]);
    }
}

