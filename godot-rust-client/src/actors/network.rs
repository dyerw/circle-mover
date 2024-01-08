use anyhow::Result;
use cm_protos::{
    cm_proto::messages::CircleMoverMessage, create_goodbye, create_hello, create_input_message,
    serialize_message,
};
use cm_sim::Input;
use godot::log::godot_error;
use tokio::sync::mpsc;

use crate::util::network::connect;

enum NetworkActorMessage {
    SendHello,
    SendGoodbye,
    SendInput(Input),
}

struct NetworkActor {
    receiver: mpsc::Receiver<NetworkActorMessage>,
    connection: quinn::Connection,
}

impl NetworkActor {
    async fn init(receiver: mpsc::Receiver<NetworkActorMessage>) -> Result<Self> {
        let connection = connect().await?;
        Ok(NetworkActor {
            receiver,
            connection,
        })
    }

    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: NetworkActorMessage) {
        let result = match msg {
            NetworkActorMessage::SendHello => self.send_hello().await,
            NetworkActorMessage::SendGoodbye => self.send_goodbye().await,
            NetworkActorMessage::SendInput(input) => self.send_input(input).await,
        };
        if let Err(e) = result {
            godot_error!("{:?}", e);
        };
    }

    async fn send_message(&self, msg: CircleMoverMessage) -> Result<()> {
        let bytes = serialize_message(msg);
        let mut send = self.connection.open_uni().await?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        Ok(())
    }

    async fn send_hello(&mut self) -> Result<()> {
        let msg = create_hello("world".to_string());
        self.send_message(msg).await
    }

    async fn send_goodbye(&mut self) -> Result<()> {
        let msg = create_goodbye("world".to_string());
        self.send_message(msg).await
    }

    async fn send_input(&mut self, input: Input) -> Result<()> {
        let msg = create_input_message(input);
        self.send_message(msg).await
    }
}

#[derive(Clone)]
pub struct NetworkActorHandle {
    sender: mpsc::Sender<NetworkActorMessage>,
}

impl NetworkActorHandle {
    pub fn new() -> Self {
        // Arbitrary channel size, look into this, handling back pressure etc
        let (sender, receiver) = mpsc::channel(256);
        tokio::spawn(async move {
            let mut actor = NetworkActor::init(receiver)
                .await
                .expect("NetworkHandle failed to init");
            actor.run().await;
        });

        Self { sender }
    }

    pub fn send_hello(&self) {
        let msg = NetworkActorMessage::SendHello;
        self.sender.try_send(msg).expect("Failed to send hello");
    }

    pub fn send_goodbye(&self) {
        let msg = NetworkActorMessage::SendGoodbye;
        self.sender.try_send(msg).expect("Failed to send goodbye");
    }

    pub fn send_input(&self, input: Input) {
        let msg = NetworkActorMessage::SendInput(input);
        self.sender.try_send(msg).expect("Failed to send input");
    }
}
