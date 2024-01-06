use std::time::Duration;

use tokio::{
    sync::{mpsc, watch},
    time,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use crate::{
    game::Game,
    tick_sequenced_stream::{GetTick, TickSequencedEvent, TickSequencedStream},
    Input,
};

#[derive(Clone)]
enum SimActorMessage {
    SendInput(Input),
}

impl GetTick for SimActorMessage {
    fn get_tick(self) -> i32 {
        match self {
            Self::SendInput(input) => input.for_tick,
        }
    }
}

struct SimActor {
    input_stream: TickSequencedStream<SimActorMessage, ReceiverStream<SimActorMessage>>,
    game: Game,
    game_sender: watch::Sender<(i32, Game)>,
}

impl SimActor {
    fn init(
        receiver: mpsc::Receiver<SimActorMessage>,
        game_sender: watch::Sender<(i32, Game)>,
        tick_length: time::Duration,
    ) -> Self {
        let input_stream = TickSequencedStream::new(ReceiverStream::new(receiver), tick_length);

        let game = Game::new();
        Self {
            input_stream,
            game,
            game_sender,
        }
    }

    async fn run(&mut self) {
        while let Some(msg_or_tick) = self.input_stream.next().await {
            match msg_or_tick {
                TickSequencedEvent::Tick { dt, number } => self.tick(dt, number),
                TickSequencedEvent::Event(msg) => self.handle_message(msg).await,
            }
        }
    }

    fn tick(&mut self, dt: Duration, current_tick: i32) {
        self.game.step(dt);
        self.game_sender
            .send_replace((current_tick, self.game.clone()));
    }

    async fn handle_message(&mut self, msg: SimActorMessage) {
        match msg {
            SimActorMessage::SendInput(input) => self.game.handle_input(input),
            // SimActorMessage::StartSim => self.start_sim(),
            // SimActorMessage::StopSim => self.stop_sim(),
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
        let (game_sender, game_receiver) = watch::channel((0, Game::new()));
        tokio::spawn(async move {
            let mut actor = SimActor::init(message_receiver, game_sender, tick_length);
            actor.run().await;
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
