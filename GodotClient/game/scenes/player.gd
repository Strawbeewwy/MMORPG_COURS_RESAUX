# player.gd — Local player controller.
# Pure GDScript — no dependency on the Rust GDExtension so the scene
# loads even before `cargo xtask editor` has compiled the .dll.
extends CharacterBody2D

const SPEED := 200.0

@onready var _hud = get_node_or_null("/root/World/DebugHUD")

func _ready() -> void:
	add_to_group("local_player")

	var shape := CapsuleShape2D.new()
	shape.radius = 12.0
	shape.height = 28.0
	$CollisionShape2D.shape = shape

	# client_id is assigned by the Broker via ClientAccepted — no need to set it manually.
	var net = get_node_or_null("/root/NetworkClient")
	if net:
		net.client_accepted.connect(_on_client_accepted)

func _on_client_accepted(client_id: int) -> void:
	print("player: Broker assigned client_id=%d" % client_id)

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

	# Send normalised movement direction — Broker encodes it as ClientInput.
	var net = get_node_or_null("/root/NetworkClient")
	if net and net.has_method("send_movement"):
		net.send_movement(dir.x, dir.y)
		if _hud and _hud.has_method("notify_send"):
			_hud.notify_send()
