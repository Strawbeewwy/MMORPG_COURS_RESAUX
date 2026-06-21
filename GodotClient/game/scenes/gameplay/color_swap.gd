# color_swap.gd
extends Node

@export var swap_interval: float = 5.0

var _timer: float = 0.0

func _ready() -> void:
	add_to_group("color_swap")

func _process(delta: float) -> void:
	_timer += delta
	if _timer >= swap_interval:
		_timer = 0.0
		_do_swap()

func _do_swap() -> void:
	var player = get_tree().get_first_node_in_group("local_player")
	if player and player.has_method("set_color_team"):
		var new_team: int = 1 - player.get_color_team()
		player.set_color_team(new_team)

func get_remaining() -> float:
	return swap_interval - _timer
