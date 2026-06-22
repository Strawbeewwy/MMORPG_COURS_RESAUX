# remote_player.gd — Visual representation of a remote player with network interpolation.
# Instantiated by EntityRegistry (Rust) when a player_joined event arrives.
# Uses position interpolation and dead reckoning for smooth movement.
extends Node2D

## Network optimization parameters (tweakable)
@export var interpolation_speed: float = 15.0  ## Higher = faster snap to server position
@export var dead_reckoning_enabled: bool = true  ## Enable client-side prediction
@export var dead_reckoning_speed: float = 180.0  ## Estimated movement speed for prediction
@export var max_prediction_time: float = 0.3  ## Max time to predict without server update
@export var snap_distance: float = 100.0  ## Distance threshold to snap instantly

## The server-assigned entity id.
var entity_id: int = 0
## The client id who controls this entity.
var client_id: int = 0

## Network state
var _target_position: Vector2 = Vector2.ZERO
var _last_velocity: Vector2 = Vector2.ZERO
var _time_since_update: float = 0.0
var _last_server_position: Vector2 = Vector2.ZERO

@onready var label: Label = $Label

func _ready() -> void:
	label.text = "E#%d (P#%d)" % [entity_id, client_id]
	_target_position = position
	_last_server_position = position

func _process(delta: float) -> void:
	_time_since_update += delta
	
	# Dead reckoning: predict position if enabled
	if dead_reckoning_enabled and _time_since_update < max_prediction_time:
		if _last_velocity.length_squared() > 0.01:
			_target_position += _last_velocity * delta
	
	# Interpolate towards target position
	var distance = position.distance_to(_target_position)
	
	if distance > snap_distance:
		# Too far: snap instantly (likely teleport or major correction)
		position = _target_position
	elif distance > 0.5:
		# Smooth interpolation
		position = position.lerp(_target_position, interpolation_speed * delta)
	else:
		# Close enough
		position = _target_position

## Called by entity_registry when server sends position update
func update_server_position(new_pos: Vector2) -> void:
	_last_server_position = _target_position
	_target_position = new_pos
	
	# Calculate velocity for dead reckoning
	if _time_since_update > 0.0:
		_last_velocity = (new_pos - _last_server_position) / _time_since_update
	
	_time_since_update = 0.0

func _draw() -> void:
	# Cyan filled circle for remote players
	draw_circle(Vector2.ZERO, 12.0, Color(0.0, 1.0, 0.9, 0.85))
	# Thin white outline
	draw_arc(Vector2.ZERO, 12.0, 0.0, TAU, 32, Color.WHITE, 1.5)
	
	# Debug: draw velocity vector if dead reckoning enabled
	if dead_reckoning_enabled and _last_velocity.length() > 1.0:
		var vel_normalized = _last_velocity.normalized() * 20.0
		draw_line(Vector2.ZERO, vel_normalized, Color(1.0, 1.0, 0.0, 0.6), 2.0)

