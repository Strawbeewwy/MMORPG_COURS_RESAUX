# projectile_manager.gd — Client-side projectile visual manager.
#
# Receives server projectile state via `projectiles_updated` signal and renders
# them as lightweight drawn circles (no dedicated scenes = minimal overhead).
#
# Data format per projectile (7 floats):
#   [id, x, y, dx, dy, color_team, alive]
#
# Projectiles that flip `alive = 0` are immediately removed.
extends Node2D

# ── Configuration ──────────────────────────────────────────────────────────────

@export var proj_radius : float = 5.0
@export var color_red   : Color = Color(1.0, 0.3, 0.3, 0.95)
@export var color_blue  : Color = Color(0.3, 0.6, 1.0, 0.95)

# ── State ──────────────────────────────────────────────────────────────────────

## projectile_id → { pos: Vector2, dir: Vector2, color: int }
var _projectiles : Dictionary = {}

# ── Init ───────────────────────────────────────────────────────────────────────

func _ready() -> void:
	add_to_group("projectile_manager")
	var net := get_node_or_null("/root/NetworkClient")
	if net:
		net.projectiles_updated.connect(_on_projectiles_updated)

# ── Signal handlers ────────────────────────────────────────────────────────────

func _on_projectiles_updated(data: PackedFloat32Array) -> void:
	const STRIDE := 7
	if data.size() % STRIDE != 0:
		push_warning("ProjectileManager: unexpected data size %d" % data.size())
		return

	var seen_ids : Dictionary = {}
	var count := data.size() / STRIDE

	for i in range(count):
		var b     := i * STRIDE
		var pid   := int(data[b + 0])
		var px    := data[b + 1]
		var py    := data[b + 2]
		var dx    := data[b + 3]
		var dy    := data[b + 4]
		var team  := int(data[b + 5])
		var alive := data[b + 6] > 0.5

		if not alive:
			_projectiles.erase(pid)
			continue

		_projectiles[pid] = { "pos": Vector2(px, py), "dir": Vector2(dx, dy), "color": team }
		seen_ids[pid] = true

	# Remove projectiles not mentioned this tick (server-side despawn).
	for pid in _projectiles.keys():
		if not seen_ids.has(pid):
			_projectiles.erase(pid)

	queue_redraw()

# ── Rendering ──────────────────────────────────────────────────────────────────

func _draw() -> void:
	for data in _projectiles.values():
		var col := color_red if data["color"] == 0 else color_blue
		var pos : Vector2 = data["pos"]
		# Draw a small circle with a directional tail.
		draw_circle(pos, proj_radius, col)
		var tail := pos - (data["dir"] as Vector2).normalized() * proj_radius * 2.5
		draw_line(pos, tail, Color(col.r, col.g, col.b, 0.4), 2.0)

# ── Public helpers ─────────────────────────────────────────────────────────────

func get_projectile_count() -> int:
	return _projectiles.size()
