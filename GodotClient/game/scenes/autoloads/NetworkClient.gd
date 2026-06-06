# NetworkClient.gd
# Thin GDScript wrapper around the Rust NetworkClient GDExtension node.
# This scene is loaded as an Autoload singleton named "NetworkClient".
extends NetworkClient   # Rust class registered via GDExtension

func _ready() -> void:
	# Override server address from an environment variable or config file
	# before the Rust ready() is called (set_server_addr must be called
	# before the scene tree is entered; use _init() for that in practice).
	pass

