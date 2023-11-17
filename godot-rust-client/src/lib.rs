use std::time::Duration;

use cm_sim::{CmSim, Game, Input as SimInput};
use godot::prelude::*;
use smol::{
    channel::{Receiver, Sender},
    Task,
};

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
    // Can be used if you need access to the RefCounted GD object
    // #[base]
    // base: Base<RefCounted>,
    sim_task: Option<Task<()>>,
    input_sender: Option<Sender<SimInput>>,
    stop_sender: Option<oneshot::Sender<()>>,
    state_receiver: Option<Receiver<Game>>,
}

#[godot_api]
impl IRefCounted for CmSimGD {
    fn init(_base: Base<RefCounted>) -> Self {
        // We don't have any channels until the sim is started
        Self {
            sim_task: None,
            input_sender: None,
            stop_sender: None,
            state_receiver: None,
        }
    }
}

#[godot_api]
impl CmSimGD {
    #[func]
    fn start_sim(&mut self) {
        godot_print!("Starting sim from rust");
        let (task, stop_chan, state_rec, input_sender) = CmSim::start(Duration::from_millis(2));

        self.input_sender = Some(input_sender);
        self.stop_sender = Some(stop_chan);
        self.sim_task = Some(task);
        self.state_receiver = Some(state_rec);
    }

    #[func]
    fn stop_sim(&self) {
        godot_print!("Stopping sim");
        // IDK figure this out
        // if let Some(t) = &self.sim_task {
        //     t.cancel();
        // }
    }

    #[func]
    fn get_latest_state(&self) -> Option<Gd<SimStateGD>> {
        let mut latest_game_message: Option<Game> = None;

        if let Some(rx) = &self.state_receiver {
            while !rx.is_empty() {
                latest_game_message = Some(rx.try_recv().unwrap());
            }
        }

        latest_game_message.map(|g| Gd::new(SimStateGD::from(g)))
    }

    #[func]
    fn add_circle(&self, pos: Vector2) {
        if let Some(input_sender) = &self.input_sender {
            if let Err(e) = input_sender.try_send(SimInput {
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
        if let Some(input_sender) = &self.input_sender {
            if let Err(e) = input_sender.try_send(SimInput {
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
}
