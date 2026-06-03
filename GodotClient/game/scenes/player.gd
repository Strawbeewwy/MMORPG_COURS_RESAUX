# player.gd — Local player controller.
# Reads WASD input, moves the CharacterBody2D, and sends the position
# to the server via the NetworkClient autoload every physics frame.
extends CharacterBody2D

const SPEED := 200.0

## Received from the server after login; set via NetworkClient.set_client_id().
var my_client_id: int = 0

@onready var network: NetworkClient = get_node("/root/NetworkClient")

func _ready() -> void:
	# If you have a login flow, set my_client_id here after authentication.
	network.set_client_id(my_client_id)

func _physics_process(delta: float) -> void:
	var direction := Vector2(
		Input.get_axis("move_left", "move_right"),
		Input.get_axis("move_up",   "move_down")
	).normalized()

	velocity = direction * SPEED
	move_and_slide()

	# Send position every frame — the Rust side rate-limits internally.
	network.send_position(position.x, position.y)

