mod actors;
mod classes;
mod util;

use std::time::Duration;

use actors::network::NetworkActorHandle;
use cm_shared_data::{Input as SimInput, InputType};
use cm_sim::{
    actor::{SimActor, SimArguments, SimMessage},
    game::Game,
};
use godot::prelude::*;
use ractor::{Actor, ActorRef};
use tokio::{runtime::Runtime, sync::watch};

use classes::{game_state::GameState, lobby_state::GLobbyState};

struct CmSimExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CmSimExtension {}

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
    fn get_game_state(&self) -> Gd<GameState> {
        let (_, game) = self.game_state_receiver.borrow().clone();
        Gd::from_object(GameState::from(game))
    }
}

#[derive(GodotClass)]
struct CmSimGD {
    #[base]
    base: Base<RefCounted>,

    runtime_ref: Option<Runtime>,
    network_handle: Option<NetworkActorHandle>,
    sim_ref: Option<SimReference>,
}

#[godot_api]
impl IRefCounted for CmSimGD {
    // TODO: Moving the contents of start_sim here causes problems but it would mean we
    // could get rid of all the Option
    fn init(base: Base<RefCounted>) -> Self {
        // We don't have any channels until the sim is started
        Self {
            base,
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
    fn get_latest_state(&mut self) -> Option<Gd<GameState>> {
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
                input_type: InputType::CreateCircle { x: pos.x, y: pos.y },
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
                input_type: InputType::SetDestination {
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

    #[func]
    fn get_lobby_state(&self) -> Option<Gd<GLobbyState>> {
        if let Some(nh) = &self.network_handle {
            let lobby = nh.get_lobby_state();
            Option::<GLobbyState>::from(lobby).map(|l| Gd::from_object(l))
        } else {
            None
        }
    }
}
