extends Control

@onready
var lobby_name_label = $MarginContainer/VBoxContainer/LobbyNameLabel

# Called when the node enters the scene tree for the first time.
func _ready():
	var lobby_name = Brain.brain.is_lobby_joined()
	lobby_name_label.text = lobby_name

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass


func _on_start_game_button_pressed():
	pass # Replace with function body.
