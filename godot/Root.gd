extends Node2D

@onready 
var sim = CmSimGD.new()

# Called when the node enters the scene tree for the first time.
func _ready():
	print("godot root _ready")
	sim.start_sim()
	sim.add_circle()
	


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	var state = sim.get_latest_state()
	if state != null:
		print(state)
		print("circle ids: ", state.circle_ids)
