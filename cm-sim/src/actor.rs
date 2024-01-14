use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use queues::{IsQueue, Queue};
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;

use crate::{game::Game, Input};

pub enum SimMessage {
    Tick,
    SendInput(Input),
    StartAt(SystemTime),
    Start,
}

pub struct SimState {
    game: Game,
    game_state_sender: watch::Sender<(i32, Game)>,
    // Hashmap as a sparse array indexed by tick
    input_buffer: HashMap<i32, Queue<Input>>,
    current_tick: i32,
    minimum_tick_duration: Duration,
}

impl SimState {
    fn tick(&mut self) {
        // Process all buffered input for this tick
        if let Some(ref mut tick_buffer) = self.input_buffer.remove(&self.current_tick) {
            while let Ok(input) = tick_buffer.remove() {
                self.game.handle_input(input);
            }
        }

        self.game.step();
        self.current_tick += 1;
        self.game_state_sender
            .send_replace((self.current_tick, self.game.clone()));
    }

    fn buffer_input(&mut self, input: Input) {
        // TODO: Check for < current_tick
        if let Some(tick_buffer) = self.input_buffer.get_mut(&input.for_tick) {
            tick_buffer.add(input).unwrap();
        } else {
            let mut queue = Queue::new();
            queue.add(input).unwrap();
            self.input_buffer.insert(input.for_tick, queue);
        }
    }
}

pub struct SimArguments {
    pub minimum_tick_duration: Duration,
    // A watch channel to publish game state to each tick
    pub game_state_sender: watch::Sender<(i32, Game)>,
}

pub struct SimActor;

#[async_trait]
impl Actor for SimActor {
    type State = SimState;
    type Msg = SimMessage;
    type Arguments = SimArguments;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(SimState {
            game_state_sender: arguments.game_state_sender,
            game: Game::new(arguments.minimum_tick_duration),
            input_buffer: HashMap::new(),
            current_tick: 0,
            minimum_tick_duration: arguments.minimum_tick_duration,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SimMessage::Tick => {
                state.tick();
            }
            SimMessage::SendInput(input) => {
                state.buffer_input(input);
            }
            SimMessage::StartAt(ts) => {
                // TODO: Validate that argument is in the future
                myself.send_after(ts.duration_since(SystemTime::now())?, || SimMessage::Start);
            }
            SimMessage::Start => {
                myself.send_interval(state.minimum_tick_duration, || SimMessage::Tick);
            }
        };
        Ok(())
    }
}
