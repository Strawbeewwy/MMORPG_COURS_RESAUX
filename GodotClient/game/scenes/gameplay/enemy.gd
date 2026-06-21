# enemy.gd — Ennemi local avec IA simple (errance + aggro).
extends CharacterBody2D

@export var team:          int   = 0
@export var move_speed:    float = 60.0
@export var health:        int   = 3
@export var wander_radius: float = 120.0
@export var aggro_range:   float = 200.0
@export var body_radius:   float = 10.0

var _target:        Vector2 = Vector2.ZERO
var _wander_timer:  float   = 0.0

func _ready() -> void:
	add_to_group("enemy")
	_pick_wander_target()

func _physics_process(delta: float) -> void:
	_wander_timer -= delta
	var player = get_tree().get_first_node_in_group("local_player")
	var dir := Vector2.ZERO
	if player and global_position.distance_to(player.global_position) < aggro_range:
		dir = (player.global_position - global_position).normalized()
	else:
		if _wander_timer <= 0.0 or global_position.distance_to(_target) < 8.0:
			_pick_wander_target()
		dir = (_target - global_position).normalized()
	velocity = dir * move_speed
	move_and_slide()

func _pick_wander_target() -> void:
	_wander_timer = randf_range(1.5, 3.5)
	var angle := randf() * TAU
	_target = global_position + Vector2(cos(angle), sin(angle)) * randf_range(40.0, wander_radius)

func _draw() -> void:
	var col := Color(1.0, 0.2, 0.2) if team == 0 else Color(0.2, 0.5, 1.0)
	draw_circle(Vector2.ZERO, body_radius, col)
	draw_arc(Vector2.ZERO, body_radius, 0.0, TAU, 24, Color.WHITE, 1.5)

func take_damage(amount: int, _source: String) -> void:
	health -= amount
	if health <= 0:
		queue_free()
