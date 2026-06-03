# debug_hud.gd — Real-time overlay for network and spatial debugging.
#
# Shows:
#   • Connection status (Connecting / Connected / Disconnected)
#   • Local player world position
#   • Number of remote entities tracked
#   • Last position update sent (rate, timestamp)
#   • Current shard id (derived from position + shard_size)
extends CanvasLayer

@export var shard_size: Vector2 = Vector2(1024.0, 1024.0)
@export var grid_count: Vector2i = Vector2i(4, 4)

# Internal state
var _connected: bool = false
var _remote_count: int = 0
var _last_pos: Vector2 = Vector2.ZERO
var _sends_this_second: int = 0
var _send_rate: int = 0
var _rate_timer: float = 0.0

@onready var _panel: PanelContainer = $Panel
@onready var _status_label: Label   = $Panel/VBox/StatusRow/StatusValue
@onready var _pos_label: Label      = $Panel/VBox/PosRow/PosValue
@onready var _shard_label: Label    = $Panel/VBox/ShardRow/ShardValue
@onready var _entities_label: Label = $Panel/VBox/EntitiesRow/EntitiesValue
@onready var _rate_label: Label     = $Panel/VBox/RateRow/RateValue

func _ready() -> void:
	var net = get_node_or_null("/root/NetworkClient")
	if net:
		# The Rust node will emit these when we add the signals properly.
		# For now we poll in _process.
		pass
	_set_status(false)

func _process(delta: float) -> void:
	# Poll local player position
	var player = get_tree().get_first_node_in_group("local_player")
	if player:
		_last_pos = player.global_position
		_pos_label.text = "%.0f , %.0f" % [_last_pos.x, _last_pos.y]
		_shard_label.text = _compute_shard_id(_last_pos)

	# Rate counter
	_rate_timer += delta
	if _rate_timer >= 1.0:
		_send_rate = _sends_this_second
		_sends_this_second = 0
		_rate_timer = 0.0
	_rate_label.text = "%d pkt/s" % _send_rate

	# Entity count (poll EntityRegistry group)
	var registry = get_tree().get_first_node_in_group("entity_registry")
	if registry and registry.has_method("get_entity_count"):
		_remote_count = registry.get_entity_count()
	_entities_label.text = str(_remote_count)

## Call from NetworkClient signals or poll-based detection.
func notify_connected() -> void:
	_set_status(true)

func notify_disconnected() -> void:
	_set_status(false)

## Call once per position packet sent (from player.gd).
func notify_send() -> void:
	_sends_this_second += 1

func _set_status(connected: bool) -> void:
	_connected = connected
	if connected:
		_status_label.text = "✅ Connected"
		_status_label.modulate = Color(0.3, 1.0, 0.4)
	else:
		_status_label.text = "🔴 Disconnected"
		_status_label.modulate = Color(1.0, 0.35, 0.35)

func _compute_shard_id(pos: Vector2) -> String:
	var cx := int(pos.x / shard_size.x)
	var cy := int(pos.y / shard_size.y)
	cx = clamp(cx, 0, grid_count.x - 1)
	cy = clamp(cy, 0, grid_count.y - 1)
	var id := cy * grid_count.x + cx
	return "shard_%d  (%d, %d)" % [id, cx, cy]

