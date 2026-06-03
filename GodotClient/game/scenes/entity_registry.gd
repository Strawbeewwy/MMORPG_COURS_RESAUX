# entity_registry.gd — Remote entity manager.
#
# Fallback-safe: extends Node2D (not the Rust EntityRegistry) so the scene
# loads without the GDExtension.  When the .dll is present, swap the
# extends to `EntityRegistry` (Rust) and remove the manual tracking below.
extends Node2D

const REMOTE_PLAYER_SCENE := preload("res://scenes/entities/remote_player.tscn")

## client_id (int) → Node2D instance
var _entities: Dictionary = {}

func _ready() -> void:
	add_to_group("entity_registry")
	var net = get_node_or_null("/root/NetworkClient")
	if net:
		net.position_received.connect(_on_position_received)
		net.player_joined.connect(_on_player_joined)
		net.player_left.connect(_on_player_left)

# ── Signal handlers ───────────────────────────────────────────────────────────

func _on_position_received(client_id: int, x: float, y: float) -> void:
	if _entities.has(client_id):
		_entities[client_id].position = Vector2(x, y)

func _on_player_joined(client_id: int) -> void:
	if _entities.has(client_id):
		return
	var node: Node2D = REMOTE_PLAYER_SCENE.instantiate()
	node.client_id = client_id
	add_child(node)
	_entities[client_id] = node

func _on_player_left(client_id: int) -> void:
	if _entities.has(client_id):
		_entities[client_id].queue_free()
		_entities.erase(client_id)

# ── Called by DebugHUD ────────────────────────────────────────────────────────

func get_entity_count() -> int:
	return _entities.size()
