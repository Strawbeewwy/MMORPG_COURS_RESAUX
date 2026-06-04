# projectile.gd — Projectile local instancie par le joueur.
extends Area2D

@export var speed:    float = 480.0
@export var lifetime: float = 2.0
@export var damage:   int   = 1

var _direction:  Vector2 = Vector2.RIGHT
var _color_team: int     = 0
var _timer:      float   = 0.0

func init(direction: Vector2, proj_speed: float, color_team: int) -> void:
	_direction  = direction.normalized()
	speed       = proj_speed
	_color_team = color_team
	modulate    = Color(1.0, 0.25, 0.25) if color_team == 0 else Color(0.25, 0.5, 1.0)

func _physics_process(delta: float) -> void:
	_timer += delta
	if _timer >= lifetime:
		queue_free()
		return
	global_position += _direction * speed * delta

func _on_body_entered(body: Node) -> void:
	if not body.is_in_group("enemy"):
		return
	if body.get("team") == _color_team:
		if body.has_method("take_damage"):
			body.take_damage(damage, "projectile")
		queue_free()

func _draw() -> void:
	var col := Color(1.0, 0.3, 0.3) if _color_team == 0 else Color(0.3, 0.6, 1.0)
	draw_circle(Vector2.ZERO, 5.0, col)
	draw_line(Vector2.ZERO, -_direction * 10.0, Color(col.r, col.g, col.b, 0.4), 2.0)
