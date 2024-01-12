mod actors;
mod util;

use std::time::Duration;

use actors::network::NetworkActorHandle;
use cm_sim::{
    actor::{SimActor, SimArguments, SimMessage},
    game::Game,
    Input as SimInput,
};
use godot::prelude::*;
use ractor::{Actor, ActorRef};
use tokio::{runtime::Runtime, sync::watch};

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

/// A sim actor bundled with a watch channel for synchonous game state access.
/// In order to poll from Godot we need the actor updating the channel.
struct SimReference {
    sim_actor: ActorRef<SimMessage>,
    game_state_receiver: watch::Receiver<(i32, Game)>,
}

impl SimReference {
    fn send_input(&self, input: SimInput) {
        self.sim_actor
            .cast(SimMessage::SendInput(input))
            .expect("Failed to send input");
    }
    fn get_current_tick(&self) -> i32 {
        let (tick, _) = *self.game_state_receiver.borrow();
        tick
    }
    fn get_game_state(&self) -> Gd<SimStateGD> {
        let (_, game) = self.game_state_receiver.borrow().clone();
        Gd::new(SimStateGD::from(game))
    }
}

#[derive(GodotClass)]
struct CmSimGD {
    runtime_ref: Option<Runtime>,
    network_handle: Option<NetworkActorHandle>,
    sim_ref: Option<SimReference>,
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
            sim_ref: None,
        }
    }
}

#[godot_api]
impl CmSimGD {
    #[func]
    fn connect_to_server(&mut self) {
        godot_print!("Connecting to server");
        let rt = Runtime::new().unwrap();
        let _enter_guard = rt.enter();

        self.network_handle = Some(NetworkActorHandle::new());

        self.runtime_ref = Some(rt);
    }

    #[func]
    fn is_connected_to_server(&self) -> bool {
        if let Some(handle) = &self.network_handle {
            return handle.is_connected();
        }
        return false;
    }

    // TODO: Moving this to init to avoid Option types causes tokio::task::spawn to panic
    #[func]
    fn start_sim(&mut self) {
        godot_print!("Starting sim from rust");

        if let Some(ref rt) = self.runtime_ref {
            let (game_state_tx, game_state_rx) =
                watch::channel((0, Game::new(Duration::from_millis(22))));
            let (actor, _actor_handle) = rt
                .block_on(Actor::spawn(
                    Some("ClientSim".to_string()),
                    SimActor,
                    // Roughly 45hz
                    SimArguments {
                        minimum_tick_duration: Duration::from_millis(22),
                        game_state_sender: game_state_tx,
                    },
                ))
                .expect("Sim failed to start");
            self.sim_ref = Some(SimReference {
                sim_actor: actor,
                game_state_receiver: game_state_rx,
            });
        }
    }

    #[func]
    fn stop_sim(&self) {
        godot_print!("Stopping sim");
        todo!();
    }

    #[func]
    fn get_latest_state(&mut self) -> Option<Gd<SimStateGD>> {
        if let Some(ref sim) = self.sim_ref {
            Some(sim.get_game_state())
        } else {
            None
        }
    }

    #[func]
    fn add_circle(&mut self, pos: Vector2) {
        if let Some(ref sim) = self.sim_ref {
            let tick = sim.get_current_tick();
            // TODO: Figure out latency for tick
            let input = SimInput {
                for_tick: tick + 1,
                player_id: 0,
                input_type: cm_sim::InputType::CreateCircle { x: pos.x, y: pos.y },
            };
            sim.send_input(input);
            if let Some(ref handle) = self.network_handle {
                handle.send_input(input);
            }
        } else {
            godot_error!("Cannot add circle, sim not started")
        }
    }

    #[func]
    fn set_destination(&mut self, circle_id: i64, pos: Vector2) {
        if let Some(ref sim) = self.sim_ref {
            let tick = sim.get_current_tick();
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
            sim.send_input(input);
            if let Some(ref handle) = self.network_handle {
                handle.send_input(input);
            }
        } else {
            godot_error!("Cannot set destination, sim not started")
        }
    }

    #[func]
    fn join_lobby(&self, name: String) {
        if let Some(ref handle) = self.network_handle {
            handle.join_lobby(name);
        }
    }

    #[func]
    fn create_lobby(&self, name: String) {
        if let Some(ref handle) = self.network_handle {
            handle.create_lobby(name);
        }
    }
}
