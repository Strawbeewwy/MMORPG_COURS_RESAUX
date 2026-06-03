# shard_boundary_debug.gd — Shard grid visualiser (@tool, works without GDExtension).
# Extends plain Node2D — no dependency on the Rust ShardBoundaryDebug class.
# When the .dll is loaded you can swap to `extends ShardBoundaryDebug` and
# remove _draw() to use the Rust implementation instead.
@tool
extends Node2D

@export var shard_size: Vector2 = Vector2(1024.0, 1024.0)
@export var grid_count: Vector2i = Vector2i(4, 4)
@export var line_color: Color = Color(0.0, 1.0, 1.0, 0.8)

func _draw() -> void:
	for x in range(grid_count.x):
		for y in range(grid_count.y):
			var origin := Vector2(x * shard_size.x, y * shard_size.y)
			draw_rect(Rect2(origin, shard_size), line_color, false, 2.0)
