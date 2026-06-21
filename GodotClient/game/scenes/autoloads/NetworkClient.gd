# NetworkClient.gd
# Thin GDScript wrapper around the Rust NetworkClient GDExtension node.
# This scene is loaded as an Autoload singleton named "NetworkClient".
extends NetworkClient   # Rust class registered via GDExtension

var session_token: String = ""
var assigned_client_id: int = 0

func _init() -> void:
	# Read Broker connection parameters from environment variables
	# (set by Launcher when spawning Godot client)
	var broker_host := OS.get_environment("BROKER_HOST")
	var broker_port := OS.get_environment("BROKER_PORT")
	
	if broker_host != "" and broker_port != "":
		var broker_addr := "%s:%s" % [broker_host, broker_port]
		set_broker_addr(broker_addr)
		print("[NetworkClient] Broker address set to: %s" % broker_addr)
	else:
		# Fallback to default localhost broker
		print("[NetworkClient] No BROKER_HOST/PORT env vars, using default 127.0.0.1:9600")
	
	# Store session token if provided
	session_token = OS.get_environment("SESSION_TOKEN")
	if session_token != "":
		print("[NetworkClient] Session token received from Launcher")

func _ready() -> void:
	# Connect to signals for tracking connection state
	client_accepted.connect(_on_client_accepted_internal)
	print("[NetworkClient] Autoload ready, Rust network thread starting...")

func _on_client_accepted_internal(client_id: int) -> void:
	assigned_client_id = client_id
	print("[NetworkClient] Client ID assigned: %d" % client_id)

func is_connected() -> bool:
	return assigned_client_id > 0

func get_session_token() -> String:
	return session_token
