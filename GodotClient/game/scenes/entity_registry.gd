# entity_registry.gd — Remote player manager.
# Connect NetworkClient signals here; the heavy lifting is done in
# the Rust EntityRegistry GDExtension node (world/mod.rs).
extends EntityRegistry   # Rust class registered via GDExtension

@onready var network: NetworkClient = get_node("/root/NetworkClient")

func _ready() -> void:
	network.position_received.connect(on_position_received)
	network.player_joined.connect(on_player_joined)
	network.player_left.connect(on_player_left)

