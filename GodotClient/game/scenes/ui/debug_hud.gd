# debug_hud.gd
extends CanvasLayer

@export var shard_size: Vector2 = Vector2(1024.0, 1024.0)
@export var grid_count: Vector2i = Vector2i(4, 4)

var _connected: bool = false
var _remote_count: int = 0
var _last_pos: Vector2 = Vector2.ZERO
var _sends_this_second: int = 0
var _send_rate: int = 0
var _rate_timer: float = 0.0

@onready var _status_label: Label   = $Panel/VBox/StatusRow/StatusValue
@onready var _pos_label: Label      = $Panel/VBox/PosRow/PosValue
@onready var _shard_label: Label    = $Panel/VBox/ShardRow/ShardValue
@onready var _entities_label: Label = $Panel/VBox/EntitiesRow/EntitiesValue
@onready var _rate_label: Label     = $Panel/VBox/RateRow/RateValue
@onready var _enemies_label: Label  = $Panel/VBox/EnemiesRow/EnemiesValue
@onready var _proj_label: Label     = $Panel/VBox/ProjRow/ProjValue
@onready var _team_label: Label     = $Panel/VBox/TeamRow/TeamValue
@onready var _score_label: Label    = $Panel/VBox/ScoreRow/ScoreValue

func _ready() -> void:
	_set_status(false)

func _process(delta: float) -> void:
	var player = get_tree().get_first_node_in_group("local_player")
	if player:
		_last_pos = player.global_position
		_pos_label.text = "%.0f , %.0f" % [_last_pos.x, _last_pos.y]
		_shard_label.text = _compute_shard_id(_last_pos)
		if player.has_method("get_color_team"):
			var t_team : int = player.get_color_team()
			_team_label.text = "RED" if t_team == 0 else "BLUE"
			_team_label.modulate = Color(1, 0.3, 0.3) if t_team == 0 else Color(0.3, 0.6, 1)
		if player.has_method("get_score"):
			_score_label.text = str(player.get_score())
	_rate_timer += delta
	if _rate_timer >= 1.0:
		_send_rate = _sends_this_second
		_sends_this_second = 0
		_rate_timer = 0.0
	_rate_label.text = "%d pkt/s" % _send_rate
	var registry = get_tree().get_first_node_in_group("entity_registry")
	if registry and registry.has_method("get_entity_count"):
		_remote_count = registry.get_entity_count()
	_entities_label.text = str(_remote_count)
	var enemy_renderer = get_tree().get_first_node_in_group("enemy_renderer")
	if enemy_renderer and enemy_renderer.has_method("get_enemy_count"):
		_enemies_label.text = str(enemy_renderer.get_enemy_count())
	var proj_mgr = get_tree().get_first_node_in_group("projectile_manager")
	if proj_mgr and proj_mgr.has_method("get_projectile_count"):
		_proj_label.text = str(proj_mgr.get_projectile_count())

func notify_connected() -> void:
	_set_status(true)

func notify_disconnected() -> void:
	_set_status(false)

func notify_send() -> void:
	_sends_this_second += 1

func _set_status(connected: bool) -> void:
	_connected = connected
	if connected:
		_status_label.text = "OK Connected"
		_status_label.modulate = Color(0.3, 1.0, 0.4)
	else:
		_status_label.text = "-- Disconnected"
		_status_label.modulate = Color(1.0, 0.35, 0.35)

func _compute_shard_id(pos: Vector2) -> String:
	var cx := int(pos.x / shard_size.x)
	var cy := int(pos.y / shard_size.y)
	cx = clamp(cx, 0, grid_count.x - 1)
	cy = clamp(cy, 0, grid_count.y - 1)
	var id := cy * grid_count.x + cx
	return "shard_%d  (%d, %d)" % [id, cx, cy]
