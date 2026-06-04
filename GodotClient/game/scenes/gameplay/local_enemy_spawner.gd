# local_enemy_spawner.gd — Spawn d'ennemis locaux (sans serveur).
# Assigner enemy_scene dans l'Inspecteur.
extends Node

@export var enemy_scene:   PackedScene
@export var enemy_count:   int     = 50
@export var spawn_radius:  float   = 600.0
@export var spawn_center:  Vector2 = Vector2(640.0, 360.0)
@export var respawn_delay: float   = 3.0

var _alive: int    = 0
var _respawn_timer: float = 0.0

func _ready() -> void:
	add_to_group("enemy_renderer")
	_spawn_all()

func _process(delta: float) -> void:
	if _alive < enemy_count:
		_respawn_timer -= delta
		if _respawn_timer <= 0.0:
			_respawn_timer = respawn_delay
			_spawn_one()

func _spawn_all() -> void:
	if enemy_scene == null:
		push_warning("LocalEnemySpawner: enemy_scene non assignee !")
		return
	for i in enemy_count:
		_spawn_one()

func _spawn_one() -> void:
	if enemy_scene == null:
		return
	var e: Node2D = enemy_scene.instantiate()
	add_child(e)
	var angle := randf() * TAU
	var dist  := randf_range(80.0, spawn_radius)
	e.global_position = spawn_center + Vector2(cos(angle), sin(angle)) * dist
	if e.get("team") != null:
		e.set("team", randi() % 2)
	e.tree_exiting.connect(_on_enemy_exiting)
	_alive += 1

func _on_enemy_exiting() -> void:
	_alive -= 1

func get_enemy_count() -> int:
	return _alive
