# shard_boundary_debug.gd — Editor + runtime shard grid visualiser.
# Attach this to a Node2D in the scene.  Works in @tool mode so you
# see the grid directly inside the Godot editor.
@tool
extends ShardBoundaryDebug   # Rust class registered via GDExtension (ui/mod.rs)

# The Rust node already implements draw() — nothing extra needed here.
# Override shard_size and grid_count from the Inspector to match your
# SpatialService QuadTree configuration.

