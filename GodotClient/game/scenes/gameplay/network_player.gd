# network_player.gd — Local player controller with network movement
extends CharacterBody2D

@export var move_speed: float = 200.0

var my_client_id: int = 0
var my_entity_id: int = 0
var is_controlled: bool = false

@onready var label: Label = $Label
@onready var camera: Camera2D = $Camera2D

func _ready() -> void:
	var net = get_node_or_null("/root/NetworkClient")
	if net:
		my_client_id = net.get_client_id()
		is_controlled = (my_client_id > 0)
		net.player_joined.connect(_on_player_joined)
		
	if label:
		label.text = "Local Player (ID: %d)" % my_client_id

func _on_player_joined(client_id: int, entity_id: int, x: float, y: float) -> void:
	# Check if this is our own entity
	if client_id == my_client_id:
		my_entity_id = entity_id
		position = Vector2(x, y)
		print("Local player spawned: entity_id=%d at (%.1f, %.1f)" % [entity_id, x, y])

func _physics_process(_delta: float) -> void:
	if not is_controlled:
		return
	
	var dir := _input_direction()
	velocity = dir * move_speed
	move_and_slide()
	
	# Send position to server if moving
	var net = get_node_or_null("/root/NetworkClient")
	if net and dir != Vector2.ZERO:
		net.send_movement(dir.x, dir.y)

func _input_direction() -> Vector2:
	var d := Vector2(
		Input.get_axis("ui_left", "ui_right"),
		Input.get_axis("ui_up", "ui_down")
	)
	return d.normalized() if d.length_squared() > 0.0 else Vector2.ZERO

func _draw() -> void:
	# Green filled circle for local player
	draw_circle(Vector2.ZERO, 14.0, Color(0.0, 1.0, 0.0, 0.9))
	# Thicker white outline
	draw_arc(Vector2.ZERO, 14.0, 0.0, TAU, 32, Color.WHITE, 2.0)
