# remote_player.gd — Visual representation of a remote player.
# Instantiated by EntityRegistry (Rust) when a player_joined event arrives.
# Uses a simple colored circle as placeholder — replace with AnimatedSprite2D later.
extends Node2D

## The server-assigned id for this remote player.
var client_id: int = 0

@onready var label: Label = $Label

func _ready() -> void:
	label.text = "P#%d" % client_id

func _draw() -> void:
	# Placeholder: cyan filled circle, radius 12
	draw_circle(Vector2.ZERO, 12.0, Color(0.0, 1.0, 0.9, 0.85))
	# Thin white outline
	draw_arc(Vector2.ZERO, 12.0, 0.0, TAU, 32, Color.WHITE, 1.5)

