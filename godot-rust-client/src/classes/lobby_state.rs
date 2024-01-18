use godot::prelude::*;

#[derive(Clone)]
pub enum LobbyState {
    NotJoined,
    Joined { name: String, players: Vec<String> },
}

#[derive(GodotClass, GodotConvert, ToGodot)]
pub struct GLobbyState {
    #[var]
    lobby_name: GString,
    #[var]
    players: Array<GString>,
}

#[godot_api]
impl GLobbyState {}

impl From<LobbyState> for Option<GLobbyState> {
    fn from(lobby_state: LobbyState) -> Self {
        match lobby_state {
            LobbyState::NotJoined => None,
            LobbyState::Joined { name, players } => {
                let mut players_arr = Array::<GString>::new();
                for p in players.iter() {
                    let gstring = GString::from(p);
                    players_arr.push(gstring);
                }
                Some(GLobbyState {
                    lobby_name: GString::from(name),
                    players: players_arr,
                })
            }
        }
    }
}
