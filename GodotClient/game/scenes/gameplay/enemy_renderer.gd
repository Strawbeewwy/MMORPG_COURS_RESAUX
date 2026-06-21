# enemy_renderer.gd — GPU-instanced enemy renderer using MultiMeshInstance2D.
#
# Uses TWO MultiMeshInstance2D children (one per colour team) so the shader
# can tint each batch independently without per-instance draw calls.
#
# Data arrives via NetworkClient signal `enemies_updated(PackedFloat32Array)`.
# Format per enemy: [id_f32, x, y, color_f32, hp_f32]   (5 floats each)
#
# Performance goal: handle 500+ enemies at 60 fps with GPU instancing.
extends Node2D

# ── Configuration ──────────────────────────────────────────────────────────────

## Radius of the enemy circle mesh (world units).
@export var enemy_radius : float = 8.0
## Tint for Red-team enemies.
@export var color_red  : Color = Color(1.0, 0.2, 0.2, 0.9)
## Tint for Blue-team enemies.
@export var color_blue : Color = Color(0.2, 0.5, 1.0, 0.9)

# ── Internal state ─────────────────────────────────────────────────────────────

var _mm_red  : MultiMeshInstance2D
var _mm_blue : MultiMeshInstance2D

# id → last known position (for death FX interpolation)
var _positions : Dictionary = {}
var _dead_ids  : Array[int] = []

# ── Init ───────────────────────────────────────────────────────────────────────

func _ready() -> void:
	add_to_group("enemy_renderer")
	_mm_red  = _make_multimesh(color_red)
	_mm_blue = _make_multimesh(color_blue)
	add_child(_mm_red)
	add_child(_mm_blue)

	var net := get_node_or_null("/root/NetworkClient")
	if net:
		net.enemies_updated.connect(_on_enemies_updated)
		net.enemy_died.connect(_on_enemy_died)

func _make_multimesh(tint: Color) -> MultiMeshInstance2D:
	var mesh := QuadMesh.new()
	mesh.size = Vector2(enemy_radius * 2.0, enemy_radius * 2.0)

	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_2D
	mm.use_colors = true
	mm.mesh = mesh
	mm.instance_count = 0

	var mat := CanvasItemMaterial.new()

	var inst := MultiMeshInstance2D.new()
	inst.multimesh = mm
	inst.material = mat
	return inst

# ── Signal handlers ────────────────────────────────────────────────────────────

## Called every server tick with a flat float array.
## Stride: 5 floats per enemy — [id, x, y, color_team, hp]
func _on_enemies_updated(data: PackedFloat32Array) -> void:
	var stride := 5
	var count  := data.size() / stride
	if data.size() % stride != 0:
		push_warning("EnemyRenderer: unexpected data size %d" % data.size())
		return

	# Separate by colour team.
	var red_transforms  : Array[Transform2D] = []
	var blue_transforms : Array[Transform2D] = []

	for i in range(count):
		var base := i * stride
		var eid   := int(data[base + 0])
		var ex    := data[base + 1]
		var ey    := data[base + 2]
		var team  := int(data[base + 3])  # 0=Red, 1=Blue
		var hp    := data[base + 4]

		if hp <= 0.0:
			continue   # skip dead enemies (EnemyDied handles cleanup)

		_positions[eid] = Vector2(ex, ey)
		var t := Transform2D.IDENTITY
		t.origin = Vector2(ex, ey)

		if team == 0:
			red_transforms.append(t)
		else:
			blue_transforms.append(t)

	_apply_transforms(_mm_red,  red_transforms,  color_red)
	_apply_transforms(_mm_blue, blue_transforms, color_blue)

## Called when an enemy is permanently removed.
func _on_enemy_died(enemy_id: int) -> void:
	# Spawn a quick death particle burst at last known position.
	if _positions.has(enemy_id):
		_spawn_death_fx(_positions[enemy_id])
		_positions.erase(enemy_id)

# ── Helpers ────────────────────────────────────────────────────────────────────

func _apply_transforms(
	inst: MultiMeshInstance2D,
	transforms: Array[Transform2D],
	tint: Color,
) -> void:
	var mm := inst.multimesh
	mm.instance_count = transforms.size()
	for i in range(transforms.size()):
		mm.set_instance_transform_2d(i, transforms[i])
		mm.set_instance_color(i, tint)

func _spawn_death_fx(pos: Vector2) -> void:
	# Simple procedural burst: 8 small circles flying outward.
	for i in range(8):
		var angle := i * TAU / 8.0
		var dir   := Vector2(cos(angle), sin(angle))
		_draw_burst_circle(pos, dir)

func _draw_burst_circle(_pos: Vector2, _dir: Vector2) -> void:
	# Placeholder — replace with a CPUParticles2D scene instance for full VFX.
	pass

# ── Public helpers ─────────────────────────────────────────────────────────────

func get_enemy_count() -> int:
	return _positions.size()
