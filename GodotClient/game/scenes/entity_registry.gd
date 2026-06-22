# entity_registry.gd — Remote entity manager.
#
# Fallback-safe: extends Node2D (not the Rust EntityRegistry) so the scene
# loads without the GDExtension.  When the .dll is present, swap the
# extends to `EntityRegistry` (Rust) and remove the manual tracking below.
extends Node2D

const REMOTE_PLAYER_SCENE := preload("res://scenes/entities/remote_player.tscn")

## entity_id (int) → Node2D instance
var _entities: Dictionary = {}

func _ready() -> void:
	add_to_group("entity_registry")
	var net = get_node_or_null("/root/NetworkClient")
	if net:
		net.position_received.connect(_on_position_received)
		net.player_joined.connect(_on_player_joined)
		net.player_left.connect(_on_player_left)

# ── Signal handlers ───────────────────────────────────────────────────────────

func _on_position_received(entity_id: int, x: float, y: float) -> void:
	if _entities.has(entity_id):
		var entity = _entities[entity_id]
		# Use smooth interpolation if available
		if entity.has_method("update_server_position"):
			entity.update_server_position(Vector2(x, y))
		else:
			entity.position = Vector2(x, y)

func _on_player_joined(client_id: int, entity_id: int, x: float, y: float) -> void:
	if _entities.has(entity_id):
		return
	var node: Node2D = REMOTE_PLAYER_SCENE.instantiate()
	node.entity_id = entity_id
	node.client_id = client_id
	node.position = Vector2(x, y)
	add_child(node)
	_entities[entity_id] = node
	print("Entity spawned: entity_id=%d, client_id=%d at (%.1f, %.1f)" % [entity_id, client_id, x, y])

func _on_player_left(entity_id: int) -> void:
	if _entities.has(entity_id):
		_entities[entity_id].queue_free()
		_entities.erase(entity_id)
		print("Entity despawned: entity_id=%d" % entity_id)

# ── Called by DebugHUD ────────────────────────────────────────────────────────

func get_entity_count() -> int:
	return _entities.size()
