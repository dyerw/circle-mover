use std::{collections::HashMap, time::Duration};

use queues::{IsQueue, Queue};
use tokio::{
    sync::{mpsc, watch},
    time::{self, interval},
};
use tokio_stream::{
    wrappers::{IntervalStream, ReceiverStream},
    StreamExt,
};

use crate::{game::Game, Input};

#[derive(Clone)]
enum SimActorMessage {
    SendInput(Input),
}

enum TickOrMessage {
    Tick,
    Message(SimActorMessage),
}

struct SimActor {
    tick_length: Duration,
    game: Game,
    game_sender: watch::Sender<(i32, Game)>,
    // Hashmap as a sparse array indexed by tick
    input_buffer: HashMap<i32, Queue<Input>>,
    current_tick: i32,
}

impl SimActor {
    fn init(game_sender: watch::Sender<(i32, Game)>, tick_length: time::Duration) -> Self {
        let game = Game::new(tick_length);
        Self {
            tick_length,
            game,
            game_sender,
            input_buffer: HashMap::new(),
            current_tick: 0,
        }
    }

    async fn run(&mut self, receiver: mpsc::Receiver<SimActorMessage>) {
        let recv_stream = ReceiverStream::new(receiver).map(|m| TickOrMessage::Message(m));
        let tick_stream =
            IntervalStream::new(interval(self.tick_length)).map(|_| TickOrMessage::Tick);
        let mut input_stream = recv_stream.merge(tick_stream);
        while let Some(tick_or_msg) = input_stream.next().await {
            match tick_or_msg {
                TickOrMessage::Tick => self.tick(),
                TickOrMessage::Message(msg) => self.handle_message(msg).await,
            }
        }
    }

    fn tick(&mut self) {
        // Process all buffered input for this tick
        if let Some(ref mut tick_buffer) = self.input_buffer.remove(&self.current_tick) {
            while let Ok(input) = tick_buffer.remove() {
                self.game.handle_input(input);
            }
        }

        self.game.step();
        self.game_sender
            .send_replace((self.current_tick, self.game.clone()));
        self.current_tick += 1
    }

    async fn handle_message(&mut self, msg: SimActorMessage) {
        match msg {
            SimActorMessage::SendInput(input) => self.buffer_input(input),
            // SimActorMessage::StartSim => self.start_sim(),
            // SimActorMessage::StopSim => self.stop_sim(),
        }
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

#[derive(Clone)]
pub struct SimActorHandle {
    message_sender: mpsc::Sender<SimActorMessage>,
    game_receiver: watch::Receiver<(i32, Game)>,
}

impl SimActorHandle {
    pub fn new(tick_length: Duration) -> Self {
        let (message_sender, message_receiver) = mpsc::channel(256);
        let (game_sender, game_receiver) = watch::channel((0, Game::new(tick_length)));
        tokio::spawn(async move {
            let mut actor = SimActor::init(game_sender, tick_length);
            actor.run(message_receiver).await;
        });

        Self {
            message_sender,
            game_receiver,
        }
    }

    pub fn send_input(&self, input: Input) {
        let msg = SimActorMessage::SendInput(input);
        self.message_sender
            .try_send(msg)
            .expect("Failed to send input");
    }

    /// Provides a clone of the latest game state
    pub fn get_latest_game_state(&mut self) -> (i32, Game) {
        self.game_receiver.borrow_and_update().to_owned()
    }
}
