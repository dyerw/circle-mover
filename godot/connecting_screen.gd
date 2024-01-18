extends Control


# Called when the node enters the scene tree for the first time.
func _ready():
	Brain.brain.connect_to_server()

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	if Brain.brain.is_connected_to_server():
		get_tree().change_scene_to_file("res://menu.tscn")
