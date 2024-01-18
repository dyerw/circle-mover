use cm_sim::game::Game;
use godot::prelude::*;

/// A version of the game state in Godot types
#[derive(GodotClass, GodotConvert, ToGodot)]
pub struct GameState {
    #[var]
    circle_ids: Array<i64>,
    #[var]
    circle_positions: Array<Vector2>,
}

#[godot_api]
impl GameState {}

impl From<Game> for GameState {
    fn from(game: Game) -> Self {
        let mut id_arr = Array::<i64>::new();
        let mut pos_array = Array::<Vector2>::new();
        for c in game.circles.iter() {
            id_arr.push(c.circle_id);
            pos_array.push(Vector2::new(c.position.x, c.position.y));
        }

        Self {
            circle_ids: id_arr,
            circle_positions: pos_array,
        }
    }
}
