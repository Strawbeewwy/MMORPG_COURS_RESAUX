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
else:
push_warning("GameWorld: /root/NetworkClient not found — running offline")
func _on_client_accepted(cid: int) -> void:
print("[GameWorld] Connected as client_id=%d" % cid)
func _on_broadcast_received(_payload: PackedByteArray) -> void:
pass