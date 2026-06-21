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
    next_shard_id: u32,
}

impl QuadTree {
    /// Build a lazily subdivided tree.
    /// The root starts as a single leaf shard and splits only when overloaded.
    pub fn new(bounds: Rect, max_depth: u8) -> Self {
        Self {
            bounds,
            depth: 0,
            max_depth,
            children: None,
            shard_id: Some(ShardId(0)),
            next_shard_id: 1,
        }
    }

    fn leaf(bounds: Rect, depth: u8, max_depth: u8, shard_id: ShardId) -> Self {
        Self {
            bounds,
            depth,
            max_depth,
            children: None,
            shard_id: Some(shard_id),
            next_shard_id: 0,
        }
    }

    fn allocate_shard_id(&mut self) -> ShardId {
        let shard_id = ShardId(self.next_shard_id);
        self.next_shard_id += 1;
        shard_id
    }

    /// Split the leaf carrying `shard_id` into 4 child leaves.
    ///
    /// Returns the newly created child shard ids, or `None` when:
    /// - the shard does not exist,
    /// - the node is already split,
    /// - the node is already at max depth.
    pub fn split_shard(&mut self, shard_id: ShardId) -> Option<[ShardId; 4]> {
        self.split_shard_inner(shard_id)
    }

    fn split_shard_inner(&mut self, shard_id: ShardId) -> Option<[ShardId; 4]> {
        if self.children.is_none() && self.shard_id == Some(shard_id) {
            if self.depth >= self.max_depth {
                return None;
            }

            let quads = self.bounds.quadrants();
            let child_shards = [
                self.allocate_shard_id(),
                self.allocate_shard_id(),
                self.allocate_shard_id(),
                self.allocate_shard_id(),
            ];

            self.children = Some(Box::new([
                Self::leaf(quads[0], self.depth + 1, self.max_depth, child_shards[0]),
                Self::leaf(quads[1], self.depth + 1, self.max_depth, child_shards[1]),
                Self::leaf(quads[2], self.depth + 1, self.max_depth, child_shards[2]),
                Self::leaf(quads[3], self.depth + 1, self.max_depth, child_shards[3]),
            ]));
            self.shard_id = None;

            return Some(child_shards);
        }

        let Some(children) = self.children.as_mut() else {
            return None;
        };

        for child in children.iter_mut() {
            if let Some(new_shards) = child.split_shard_inner(shard_id) {
                return Some(new_shards);
            }
        }

        None
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

