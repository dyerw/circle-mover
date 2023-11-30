pub mod game;
use game::Game;

use std::time::Duration;

use nalgebra::point;
use std::sync::{Arc, Mutex};
use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    },
    time,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum InputType {
    CreateCircle { x: f32, y: f32 },
    SetDestination { circle_id: i64, x: f32, y: f32 },
}

#[derive(Debug)]
pub struct Input {
    pub for_tick: u16,
    pub player_id: u8,
    pub input_type: InputType,
}

const INPUT_BUFFER_SIZE: usize = 10;

pub struct CmSim {
    game: Game,
    current_tick: u16,
    // index 0 is for the current tick, 1 is current + 1, etc
    input_buffer: [Vec<Input>; INPUT_BUFFER_SIZE],
}

impl Default for CmSim {
    fn default() -> Self {
        Self::new()
    }
}

impl CmSim {
    pub fn new() -> CmSim {
        CmSim {
            game: Game::new(),
            current_tick: 0,
            input_buffer: std::array::from_fn(|_| Vec::new()),
        }
    }

    async fn receive_input(mut input_rx: Receiver<Input>, sim_ref: Arc<Mutex<CmSim>>) {
        loop {
            // Wait to receive input and then acquire a lock on the sim state
            let input = input_rx.recv().await;
            let mut sim_lock = sim_ref.lock().unwrap();

            if let Some(input) = input {
                if input.for_tick >= sim_lock.current_tick {
                    let current_tick = sim_lock.current_tick;
                    let tick_vector = &mut sim_lock.input_buffer
                        [Into::<usize>::into(input.for_tick - current_tick)];
                    tick_vector.push(input);
                } else {
                    // TODO: What to do if receiving an input for a previous tick?
                }
            }
        }
    }

    async fn run_sim(
        tick_length: Duration,
        sim_ref: Arc<Mutex<CmSim>>,
        state_tx: watch::Sender<(u16, Game)>,
    ) {
        // By default Interval has the "Burst" MissedTickStragegy
        // TODO: Find some way to be notified if this happens
        let mut interval = time::interval(tick_length);
        loop {
            interval.tick().await;
            // Acquire a lock and tick
            let mut sim_lock = sim_ref.lock().unwrap();
            sim_lock.tick(tick_length.as_secs_f32() / 1.0);

            sim_lock.current_tick += 1;

            // Get a copy of the game state and send it to the channel
            let game = sim_lock.game.clone();
            // TODO: Handle game channel full
            state_tx.send((sim_lock.current_tick, game)).unwrap();
        }
    }

    pub fn start(
        tick_length: Duration,
    ) -> (
        watch::Receiver<(u16, Game)>,
        Sender<Input>,
        CancellationToken,
    ) {
        let (input_tx, input_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel::<(u16, Game)>((0, Game::new()));

        let cancellation_token = CancellationToken::new();
        let ct_input_clone = cancellation_token.clone();
        let ct_tick_clone = cancellation_token.clone();

        let cm_sim = Arc::new(Mutex::new(CmSim::new()));
        let input_task_sim_ref = cm_sim.clone();
        let tick_task_sim_ref = cm_sim.clone();

        // Spawn two tasks, one to receive input and another to tick the sim
        // each has a reference counted ref to the sim mutex to keep state
        // access synchronized
        tokio::spawn(async move {
            select! {
                _ = ct_input_clone.cancelled() => {}
                _ = Self::receive_input(input_rx, input_task_sim_ref) => {}
            }
        });

        tokio::spawn(async move {
            select! {
                _ = ct_tick_clone.cancelled() => {}
                _ = Self::run_sim(tick_length, tick_task_sim_ref, state_tx) => {}
            }
        });

        (state_rx, input_tx, cancellation_token)
    }

    // Processes the current tick's inputs and shifts the input buffer array
    fn process_current_tick_inputs(&mut self) {
        println!("{:?}", self.input_buffer);
        for input in self.input_buffer[0].iter() {
            match input.input_type {
                InputType::CreateCircle { x, y } => {
                    self.game.add_circle(point![x, y], input.player_id)
                }
                InputType::SetDestination { circle_id, x, y } => {
                    if self.game.circle_owned_by(circle_id, input.player_id) {
                        self.game.set_destination(point![x, y], circle_id)
                    }
                }
            }
        }
        // Rotate the current tick's buffer to the end and clear it
        self.input_buffer.rotate_right(INPUT_BUFFER_SIZE - 1);
        self.input_buffer[INPUT_BUFFER_SIZE - 1] = Vec::new();
    }

    fn tick(&mut self, ds: f32) {
        self.process_current_tick_inputs();
        self.game.step(ds);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn it_stops() {
        let tick_duration = Duration::from_millis(20);

        let (mut state_rx, _input_tx, ct) = CmSim::start(tick_duration);

        // Let the sim tick 4 times then stop it
        time::sleep(tick_duration * 4).await;
        ct.cancel();
        // Check that with more time it doesn't continue running
        time::sleep(tick_duration * 2).await;

        let (tick, _game) = state_rx.borrow_and_update().clone();

        assert_eq!(tick, 4)
    }

    #[tokio::test]
    async fn it_adds_circles() {
        let tick_duration = Duration::from_millis(20);

        let (mut state_rx, input_tx, _ct) = CmSim::start(tick_duration);

        let _ = input_tx
            .send(Input {
                for_tick: 4,
                player_id: 0,
                input_type: InputType::CreateCircle { x: 0.0, y: 0.0 },
            })
            .await;

        time::sleep(tick_duration * 5).await;

        let (tick, game) = state_rx.borrow_and_update().clone();

        assert_eq!(tick, 5);
        assert_eq!(game.circles.len(), 1);

        let _ = input_tx
            .send(Input {
                for_tick: 6,
                player_id: 0,
                input_type: InputType::CreateCircle { x: 0.0, y: 0.0 },
            })
            .await;
    }

    #[test]
    fn circles_move() {
        //     smol::block_on(async {
        //         let (task, stop_chan, state_rec, input_sender) =
        //             CmSim::start(Duration::from_millis(250));

        //         let _ = input_sender
        //             .send(Input {
        //                 player_id: 0,
        //                 input_type: InputType::CreateCircle { x: 0.0, y: 0.0 },
        //             })
        //             .await;

        //         let _ = input_sender
        //             .send(Input {
        //                 player_id: 0,
        //                 input_type: InputType::SetDestination {
        //                     circle_id: 0,
        //                     x: 5.0,
        //                     y: 0.0,
        //                 },
        //             })
        //             .await;

        //         Timer::after(Duration::from_secs(5)).await;
        //         let _ = stop_chan.send(());

        //         let mut states: Vec<Game> = vec![];

        //         while !state_rec.is_empty() {
        //             let state = state_rec.try_recv().unwrap();
        //             states.push(state);
        //         }

        //         println!("{:?}", states.last());
        //         task.await
        //     })
    }
}
