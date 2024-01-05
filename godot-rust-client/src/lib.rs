mod actors;
mod util;

use std::time::Duration;

use actors::network::NetworkActorHandle;
use cm_sim::{game::Game, CmSim, Input as SimInput};
use godot::prelude::*;
use tokio::{
    runtime::Runtime,
    sync::{mpsc::Sender, watch::Receiver},
};
use tokio_util::sync::CancellationToken;

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
    input_tx: Option<Sender<SimInput>>,
    state_rx: Option<Receiver<(u16, Game)>>,
    cancellation_token: Option<CancellationToken>,
    runtime_ref: Option<Runtime>,
    network_handle: Option<NetworkActorHandle>,
}

#[godot_api]
impl IRefCounted for CmSimGD {
    // TODO: Moving the contents of start_sim here causes problems but it would mean we
    // could get rid of all the Option
    fn init(_base: Base<RefCounted>) -> Self {
        // We don't have any channels until the sim is started
        Self {
            input_tx: None,
            state_rx: None,
            cancellation_token: None,
            runtime_ref: None,
            network_handle: None,
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

        self.runtime_ref = Some(rt);

        let (state_rx, input_tx, ct) = CmSim::start(Duration::from_millis(2));
        self.input_tx = Some(input_tx);
        self.state_rx = Some(state_rx);
        self.cancellation_token = Some(ct.clone());
    }

    #[func]
    fn stop_sim(&self) {
        godot_print!("Stopping sim");
        if let Some(ct) = &self.cancellation_token {
            ct.cancel();
        }
    }

    #[func]
    fn get_latest_state(&self) -> Option<Gd<SimStateGD>> {
        if let Some(rx) = &self.state_rx {
            let (_, game) = rx.borrow().clone();
            Some(Gd::new(SimStateGD::from(game)))
        } else {
            // Error can't get latest game from un init
            None
        }
    }

    #[func]
    fn add_circle(&self, pos: Vector2) {
        if let Some((input_tx, state_rx)) = self.input_tx.as_ref().zip(self.state_rx.as_ref()) {
            let (tick, _) = *state_rx.borrow();
            // TODO: Figure out latency for tick
            if let Err(e) = input_tx.try_send(SimInput {
                for_tick: tick + 1,
                player_id: 0,
                input_type: cm_sim::InputType::CreateCircle { x: pos.x, y: pos.y },
            }) {
                godot_error!("Add Circle send error: {:?}", e)
            }
        } else {
            godot_error!("Cannot add circle, sim not started")
        }
    }

    #[func]
    fn set_destination(&self, circle_id: i64, pos: Vector2) {
        if let Some((input_tx, state_rx)) = self.input_tx.as_ref().zip(self.state_rx.as_ref()) {
            let (tick, _) = *state_rx.borrow();
            if let Err(e) = input_tx.try_send(SimInput {
                for_tick: tick + 1,
                player_id: 0,
                input_type: cm_sim::InputType::SetDestination {
                    circle_id,
                    x: pos.x,
                    y: pos.y,
                },
            }) {
                godot_error!("SetDestination send error: {:?}", e)
            }
        } else {
            godot_error!("Cannot set destination, sim not started")
        }
    }

    // QUIC/protobuf test fns
    #[func]
    fn say_hello(&self) {
        if let Some(ref handle) = self.network_handle {
            godot_print!("Sending hello");
            handle.send_hello();
        }
    }

    #[func]
    fn say_goodbye(&self) {
        if let Some(ref handle) = self.network_handle {
            godot_print!("Sending goodbye");
            handle.send_goodbye();
        }
    }
}
