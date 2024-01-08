mod actors;
mod util;

use std::time::Duration;

use actors::network::NetworkActorHandle;
use cm_sim::{actor::SimActorHandle, game::Game, Input as SimInput};
use godot::prelude::*;
use tokio::runtime::Runtime;

struct CmSimExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CmSimExtension {}

#[derive(GodotClass, GodotConvert, ToGodot)]
pub struct SimStateGD {
    #[var]
    circle_ids: Array<i64>,
    #[var]
    circle_positions: Array<Vector2>,
}

#[godot_api]
impl SimStateGD {}

impl From<Game> for SimStateGD {
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

#[derive(GodotClass)]
struct CmSimGD {
    runtime_ref: Option<Runtime>,
    network_handle: Option<NetworkActorHandle>,
    sim_handle: Option<SimActorHandle>,
}

#[godot_api]
impl IRefCounted for CmSimGD {
    // TODO: Moving the contents of start_sim here causes problems but it would mean we
    // could get rid of all the Option
    fn init(_base: Base<RefCounted>) -> Self {
        // We don't have any channels until the sim is started
        Self {
            runtime_ref: None,
            network_handle: None,
            sim_handle: None,
        }
    }
}

#[godot_api]
impl CmSimGD {
    // TODO: Moving this to init to avoid Option types causes tokio::task::spawn to panic
    #[func]
    fn start_sim(&mut self) {
        godot_print!("Starting sim from rust");

        let rt = Runtime::new().unwrap();
        let _enter_guard = rt.enter();

        self.network_handle = Some(NetworkActorHandle::new());
        self.sim_handle = Some(SimActorHandle::new(Duration::from_millis(5)));

        self.runtime_ref = Some(rt);
    }

    #[func]
    fn stop_sim(&self) {
        godot_print!("Stopping sim");
        todo!();
    }

    #[func]
    fn get_latest_state(&mut self) -> Option<Gd<SimStateGD>> {
        if let Some(ref mut sim_handle) = self.sim_handle {
            let (_, game) = sim_handle.get_latest_game_state();
            Some(Gd::new(SimStateGD::from(game)))
        } else {
            None
        }
    }

    #[func]
    fn add_circle(&mut self, pos: Vector2) {
        if let Some(ref mut sim_handle) = self.sim_handle {
            let (tick, _) = sim_handle.get_latest_game_state();
            // TODO: Figure out latency for tick
            let input = SimInput {
                for_tick: tick + 1,
                player_id: 0,
                input_type: cm_sim::InputType::CreateCircle { x: pos.x, y: pos.y },
            };
            sim_handle.send_input(input);
            if let Some(ref handle) = self.network_handle {
                handle.send_input(input);
            }
        } else {
            godot_error!("Cannot add circle, sim not started")
        }
    }

    #[func]
    fn set_destination(&mut self, circle_id: i64, pos: Vector2) {
        if let Some(ref mut sim_handle) = self.sim_handle {
            let (tick, _) = sim_handle.get_latest_game_state();
            let input = SimInput {
                // FIXME: Actually deal with latency
                for_tick: tick + 1,
                player_id: 0,
                input_type: cm_sim::InputType::SetDestination {
                    circle_id,
                    x: pos.x,
                    y: pos.y,
                },
            };
            sim_handle.send_input(input);
            if let Some(ref handle) = self.network_handle {
                handle.send_input(input);
            }
        } else {
            godot_error!("Cannot set destination, sim not started")
        }
    }

    #[func]
    fn join_lobby(&self) {
        if let Some(ref handle) = self.network_handle {
            handle.join_lobby("LOBBY".to_string());
        }
    }

    #[func]
    fn create_lobby(&self) {
        if let Some(ref handle) = self.network_handle {
            handle.create_lobby("LOBBY".to_string());
        }
    }
}
