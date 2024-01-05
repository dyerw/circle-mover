extends Node2D

@onready 
var sim = CmSimGD.new()

var circle_scn = preload("res://circle.tscn")

var circles_by_id = {}

func _ready():
	print("godot root _ready")
	sim.start_sim()
	print("sim started")

func _process(dt):
	var state = sim.get_latest_state()
	# The Godot process is ticking faster than the sim,
	# we'll get null here if there hasn't been any updates
	if state != null:
		for i in range(state.circle_ids.size()):
			var circle_id = state.circle_ids[i]
			var circle_node: Node2D = circles_by_id.get(circle_id)
			if circle_node == null:
				print("Adding circle ", circle_id)
				circle_node = circle_scn.instantiate()
				circles_by_id[circle_id] = circle_node
				add_child(circle_node)
			circle_node.position = state.circle_positions[i]

func _input(event):
	# Mouse in viewport coordinates.
	if event is InputEventMouseButton:
		var view_to_world = get_canvas_transform().affine_inverse()
		var world_pos = view_to_world * event.position
		if event.button_index == MOUSE_BUTTON_RIGHT and event.pressed:
			sim.add_circle(world_pos)
			sim.say_hello()
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			sim.say_goodbye()
			for id in circles_by_id.keys():
				sim.set_destination(id, world_pos)
