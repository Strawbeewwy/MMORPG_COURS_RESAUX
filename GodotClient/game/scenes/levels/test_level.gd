# test_level.gd — World bounds and background grid for the test level.
# Draws a checkerboard background so movement is visible, and four
# StaticBody2D walls that keep the player inside the world.
@tool
extends Node2D

## Total world size in pixels — should match SpatialService QuadTree bounds.
@export var world_size: Vector2 = Vector2(4096.0, 4096.0)
## Tile size for the checkerboard background.
@export var tile_size: float = 128.0

func _draw() -> void:
	_draw_checkerboard()
	_draw_world_border()

func _draw_checkerboard() -> void:
	var cols := int(world_size.x / tile_size)
	var rows := int(world_size.y / tile_size)
	var col_a := Color(0.18, 0.18, 0.22)
	var col_b := Color(0.22, 0.22, 0.28)
	for x in range(cols):
		for y in range(rows):
			var c := col_a if (x + y) % 2 == 0 else col_b
			draw_rect(Rect2(Vector2(x, y) * tile_size, Vector2.ONE * tile_size), c)

func _draw_world_border() -> void:
	draw_rect(Rect2(Vector2.ZERO, world_size), Color(1.0, 0.4, 0.1), false, 3.0)

