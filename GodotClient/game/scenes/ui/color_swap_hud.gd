# color_swap_hud.gd — Overlay HUD pour le countdown colorswap + badge equipe.
extends CanvasLayer

const COLOR_RED  := Color(0.95, 0.25, 0.25)
const COLOR_BLUE := Color(0.25, 0.55, 1.0)

var _team_label:  Label
var _timer_bar:   ProgressBar
var _flash_rect:  ColorRect
var _swap_node:   Node

func _ready() -> void:
	_build_ui()
	_swap_node = get_tree().get_first_node_in_group("color_swap")
	# Fallback : chercher par nom dans la scene parente
	if not _swap_node:
		_swap_node = get_node_or_null("/root/World/ColorSwap")

func _build_ui() -> void:
	var vbox := VBoxContainer.new()
	vbox.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	vbox.custom_minimum_size = Vector2(160, 0)
	vbox.offset_left   = -170
	vbox.offset_top    = 10
	vbox.offset_right  = -10
	vbox.offset_bottom = 80
	add_child(vbox)
	_team_label = Label.new()
	_team_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_team_label.add_theme_font_size_override("font_size", 16)
	vbox.add_child(_team_label)
	var bar_label := Label.new()
	bar_label.text = "Next swap"
	bar_label.add_theme_font_size_override("font_size", 11)
	vbox.add_child(bar_label)
	_timer_bar = ProgressBar.new()
	_timer_bar.min_value = 0.0
	_timer_bar.max_value = 5.0
	_timer_bar.value = 5.0
	_timer_bar.custom_minimum_size = Vector2(150, 14)
	_timer_bar.show_percentage = false
	vbox.add_child(_timer_bar)
	_flash_rect = ColorRect.new()
	_flash_rect.color = Color.TRANSPARENT
	_flash_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	_flash_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_flash_rect)

func _process(_delta: float) -> void:
	var player = get_tree().get_first_node_in_group("local_player")
	if player and player.has_method("get_color_team"):
		var team : int = player.get_color_team()
		if team == 0:
			_team_label.text     = "RED TEAM"
			_team_label.modulate = COLOR_RED
			_timer_bar.modulate  = COLOR_RED
		else:
			_team_label.text     = "BLUE TEAM"
			_team_label.modulate = COLOR_BLUE
			_timer_bar.modulate  = COLOR_BLUE
	# Lire le temps restant depuis ColorSwap si disponible
	if not _swap_node:
		_swap_node = get_node_or_null("/root/World/ColorSwap")
	if _swap_node and _swap_node.has_method("get_remaining"):
		_timer_bar.value = _swap_node.get_remaining()
