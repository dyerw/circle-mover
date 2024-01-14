extends Control

@onready
var lobby_name_text_edit := $MarginContainer/VBoxContainer/LobbyNameTextEdit

@onready
var player_name_text_edit := $MarginContainer/VBoxContainer/PlayerNameTextEdit

# Called when the node enters the scene tree for the first time.
func _ready():
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	var lobby_name = Brain.brain.is_lobby_joined()
	if lobby_name:
		get_tree().change_scene_to_file("res://lobby.tscn")


func _on_join_lobby_button_pressed():
	if lobby_name_text_edit.text != "":
		Brain.brain.join_lobby(lobby_name_text_edit.text)


func _on_create_lobby_button_pressed():
	if lobby_name_text_edit.text != "":
		Brain.brain.create_lobby(lobby_name_text_edit.text)
