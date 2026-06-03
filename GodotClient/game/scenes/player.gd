# player.gd — Local player controller.
# Pure GDScript — no dependency on the Rust GDExtension so the scene
# loads even before `cargo xtask editor` has compiled the .dll.
extends CharacterBody2D

const SPEED := 200.0

var my_client_id: int = 1  # Hardcoded for the test level — replace post-login.

@onready var _hud = get_node_or_null("/root/World/DebugHUD")

func _ready() -> void:
	add_to_group("local_player")

	# Capsule collision shape — created in code so the scene has no inline resource dep.
	var shape := CapsuleShape2D.new()
	shape.radius = 12.0
	shape.height = 28.0
	$CollisionShape2D.shape = shape

	# Try to reach the Rust NetworkClient; silently skip if .dll not loaded yet.
	var net = get_node_or_null("/root/NetworkClient")
	if net and net.has_method("set_client_id"):
		net.set_client_id(my_client_id)

func _draw() -> void:
	# Bright green circle — visible without any sprite asset.
	draw_circle(Vector2.ZERO, 12.0, Color(0.2, 1.0, 0.3, 0.9))
	draw_arc(Vector2.ZERO, 12.0, 0.0, TAU, 32, Color.WHITE, 2.0)
	# Direction indicator dot
	draw_circle(Vector2(0, -10), 4.0, Color.WHITE)

func _physics_process(_delta: float) -> void:
	var dir := Vector2(
		Input.get_axis("ui_left",  "ui_right"),
		Input.get_axis("ui_up",    "ui_down")
	).normalized()

	velocity = dir * SPEED
	move_and_slide()
	queue_redraw()  # redraw direction dot each frame

	# Send position — fail silently if not connected yet.
	var net = get_node_or_null("/root/NetworkClient")
	if net and net.has_method("send_position"):
		net.send_position(position.x, position.y)
		if _hud and _hud.has_method("notify_send"):
			_hud.notify_send()
