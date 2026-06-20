# game_world.gd — Coordinateur de scène 5SecsSwap (optionnel).
# Ce script est attaché au noeud World si vous utilisez game_world.gd
# directement, sinon world.tscn le câble lui-meme via l'inspecteur.
extends Node2D

func _ready() -> void:
	var net := get_node_or_null("/root/NetworkClient")
	if net:
		net.broadcast_received.connect(_on_broadcast_received)
		net.client_accepted.connect(_on_client_accepted)
		
		var hud := get_node_or_null("DebugHUD")
		if hud and hud.has_method("notify_connected"):
			hud.notify_connected()
		
		# Auto-connect triggered by Launcher via environment variables
		# NetworkClient._init() already configured broker address
		print("[GameWorld] Network client ready, waiting for broker connection...")
	else:
		push_warning("GameWorld: /root/NetworkClient not found — running offline")

func _on_client_accepted(cid: int) -> void:
	print("[GameWorld] Connected to broker as client_id=%d" % cid)
	
	# Client is now connected and receiving broadcasts
	# Game state synchronization can begin
	var net := get_node_or_null("/root/NetworkClient")
	if net:
		var token := net.call("get_session_token") if net.has_method("get_session_token") else ""
		if token != "":
			print("[GameWorld] Session authenticated via launcher token")

func _on_broadcast_received(_payload: PackedByteArray) -> void:
	# Handle WorldUpdate broadcasts from GameServer
	# Payload contains serialized entity positions, events, etc.
	pass