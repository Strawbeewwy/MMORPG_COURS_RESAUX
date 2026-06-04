# player_local.gd — 5SecsSwap — joueur local
# Deplacement 8 directions, _facing = derniere direction tenue.
# Tir/melee toujours dans _facing. Dash dans direction du mouvement.
extends CharacterBody2D

# ── Parametres configurables (Inspecteur) ────────────────────
@export var move_speed:        float = 200.0
@export var dash_speed:        float = 600.0
@export var dash_duration:     float = 0.15
@export var dash_cooldown:     float = 0.8
@export var shoot_cooldown:    float = 0.20
@export var melee_range:       float = 52.0
@export var melee_cooldown:    float = 0.45
@export var projectile_speed:  float = 480.0
@export var score_per_shoot:   int   = 5
@export var score_per_melee:   int   = 10
@export var projectile_scene:  PackedScene
@export var melee_vfx_scene:   PackedScene

# ── Etat interne ──────────────────────────────────────────────
var _facing:     Vector2 = Vector2.RIGHT
var _color_team: int     = 0
var _score:      int     = 0
var _is_dashing: bool    = false
var _dash_vel:   Vector2 = Vector2.ZERO
var _dash_timer: float   = 0.0
var _dash_cd:    float   = 0.0
var _shoot_cd:   float   = 0.0
var _melee_cd:   float   = 0.0

# ── Init ──────────────────────────────────────────────────────
func _ready() -> void:
	add_to_group("local_player")
	if not has_node("Camera2D"):
		var cam := Camera2D.new()
		add_child(cam)
		cam.make_current()
	modulate = _team_color()

# ── Boucle physique ───────────────────────────────────────────
func _physics_process(delta: float) -> void:
	_tick_timers(delta)
	if _is_dashing:
		velocity = _dash_vel
		move_and_slide()
		queue_redraw()
		return
	var dir := _input_direction()
	if dir != Vector2.ZERO:
		_facing = dir
	velocity = dir * move_speed
	move_and_slide()
	queue_redraw()
	if Input.is_action_just_pressed("ui_dash") and _dash_cd <= 0.0:
		_do_dash()
	if Input.is_action_just_pressed("ui_shoot") and _shoot_cd <= 0.0:
		_do_shoot()
	if Input.is_action_just_pressed("ui_melee") and _melee_cd <= 0.0:
		_do_melee()

# ── Helpers internes ──────────────────────────────────────────
func _input_direction() -> Vector2:
	var d := Vector2(
		Input.get_axis("ui_left", "ui_right"),
		Input.get_axis("ui_up",   "ui_down")
	)
	return d.normalized() if d.length_squared() > 0.0 else Vector2.ZERO

func _tick_timers(delta: float) -> void:
	_dash_cd  = max(0.0, _dash_cd  - delta)
	_shoot_cd = max(0.0, _shoot_cd - delta)
	_melee_cd = max(0.0, _melee_cd - delta)
	if _is_dashing:
		_dash_timer -= delta
		if _dash_timer <= 0.0:
			_is_dashing = false

func _do_dash() -> void:
	var dir := _input_direction()
	if dir == Vector2.ZERO:
		dir = _facing
	_dash_vel   = dir * dash_speed
	_is_dashing = true
	_dash_timer = dash_duration
	_dash_cd    = dash_cooldown

func _do_shoot() -> void:
	_shoot_cd = shoot_cooldown
	if projectile_scene == null:
		push_warning("player_local: projectile_scene non assignee !")
		return
	var proj: Node2D = projectile_scene.instantiate()
	get_tree().current_scene.add_child(proj)
	proj.global_position = global_position + _facing * 20.0
	if proj.has_method("init"):
		proj.init(_facing, projectile_speed, _color_team)

func _do_melee() -> void:
	_melee_cd = melee_cooldown
	if melee_vfx_scene:
		var vfx: Node2D = melee_vfx_scene.instantiate()
		get_tree().current_scene.add_child(vfx)
		vfx.global_position = global_position + _facing * (melee_range * 0.5)
	for body in get_tree().get_nodes_in_group("enemy"):
		if body.global_position.distance_to(global_position) <= melee_range:
			if body.get("team") == _color_team and body.has_method("take_damage"):
				body.take_damage(1, "melee")
				_score += score_per_melee

# ── Rendu ─────────────────────────────────────────────────────
func _draw() -> void:
	var col := _team_color()
	draw_circle(Vector2.ZERO, 13.0, col)
	draw_arc(Vector2.ZERO, 13.0, 0.0, TAU, 32, Color.WHITE, 2.0)
	draw_circle(_facing * 11.0, 4.0, Color.WHITE)
	if _melee_cd > melee_cooldown - 0.15:
		_draw_melee_cone(col)
	if _is_dashing:
		var a := _dash_timer / dash_duration
		draw_arc(Vector2.ZERO, 18.0, 0.0, TAU, 32, Color(col.r, col.g, col.b, a * 0.6), 3.0)

func _draw_melee_cone(col: Color) -> void:
	var half := PI / 3.0
	var base := _facing.angle()
	var steps := 12
	for i in range(steps):
		var a0 := base - half + (2.0 * half) * float(i)       / steps
		var a1 := base - half + (2.0 * half) * float(i + 1)   / steps
		draw_line(Vector2(cos(a0), sin(a0)) * melee_range,
		Vector2(cos(a1), sin(a1)) * melee_range,
		Color(col.r, col.g, col.b, 0.7), 2.0)
	draw_line(Vector2.ZERO, Vector2(cos(base - half), sin(base - half)) * melee_range, Color(col.r, col.g, col.b, 0.45), 1.5)
	draw_line(Vector2.ZERO, Vector2(cos(base + half), sin(base + half)) * melee_range, Color(col.r, col.g, col.b, 0.45), 1.5)

# ── API publique ──────────────────────────────────────────────
func _team_color() -> Color:
	return Color(1.0, 0.25, 0.25) if _color_team == 0 else Color(0.25, 0.5, 1.0)

func get_color_team() -> int:
	return _color_team

func set_color_team(new_team: int) -> void:
	_color_team = new_team
	modulate = _team_color()
	queue_redraw()

func get_score() -> int:
	return _score
