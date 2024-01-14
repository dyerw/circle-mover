use anyhow::Result;
use cm_protos::{
    cm_proto::messages::{
        circle_mover_message::SubMessage, lobby_message::LobbySubMessage, CircleMoverMessage,
        LobbyJoined, LobbyMessage,
    },
    create_create_lobby, create_input_message, create_join_lobby, read_message, serialize_message,
};
use cm_sim::Input;
use godot::log::{godot_error, godot_print};
use tokio::sync::{mpsc, watch};

use crate::util::network::connect;

enum NetworkActorMessage {
    SendInput(Input),
    JoinLobby { name: String },
    CreateLobby { name: String },
}

struct NetworkActor {
    receiver: mpsc::Receiver<NetworkActorMessage>,
    connection: quinn::Connection,
}

impl NetworkActor {
    async fn init(
        connection: quinn::Connection,
        receiver: mpsc::Receiver<NetworkActorMessage>,
    ) -> Result<Self> {
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
            NetworkActorMessage::SendInput(input) => self.send_input(input).await,
            NetworkActorMessage::CreateLobby { name } => self.send_create_lobby(name).await,
            NetworkActorMessage::JoinLobby { name } => self.send_join_lobby(name).await,
        };
        if let Err(e) = result {
            godot_error!("{:?}", e);
        };
    }

    async fn send_create_lobby(&mut self, name: String) -> Result<()> {
        let msg = create_create_lobby(name);
        self.send_message(msg).await
    }

    async fn send_join_lobby(&mut self, name: String) -> Result<()> {
        let msg = create_join_lobby(name);
        self.send_message(msg).await
    }

    async fn send_input(&mut self, input: Input) -> Result<()> {
        let msg = create_input_message(input);
        self.send_message(msg).await
    }

    async fn send_message(&self, msg: CircleMoverMessage) -> Result<()> {
        let bytes = serialize_message(msg);
        let mut send = self.connection.open_uni().await?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct NetworkActorHandle {
    sender: mpsc::Sender<NetworkActorMessage>,
    ready: watch::Receiver<bool>,
    lobby: watch::Receiver<Option<String>>,
}

impl NetworkActorHandle {
    pub fn new() -> Self {
        // Arbitrary channel size, look into this, handling back pressure etc
        let (sender, receiver) = mpsc::channel(256);
        let (ready_tx, ready) = watch::channel(false);
        let (lobby_tx, lobby) = watch::channel(None);
        tokio::spawn(async move {
            let connection = connect().await.expect("Cannot connect to server");
            let connection_clone = connection.clone();

            let mut actor = NetworkActor::init(connection, receiver)
                .await
                .expect("NetworkHandle failed to init");
            ready_tx.send_replace(true);

            // FIXME: None of this is scalable at all to receiving more messages
            tokio::spawn(async move {
                loop {
                    match read_message(&connection_clone).await {
                        Ok(msg) => {
                            godot_print!("{:?}", msg);
                            match msg {
                                CircleMoverMessage {
                                    sub_message:
                                        Some(SubMessage::LobbyMessage(LobbyMessage {
                                            lobby_sub_message:
                                                Some(LobbySubMessage::LobbyJoined(LobbyJoined { name })),
                                        })),
                                } => {
                                    lobby_tx.send_replace(Some(name));
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            godot_error!("Failed: {}", e.to_string());
                        }
                    }
                }
            });
            actor.run().await;
        });

        Self {
            sender,
            ready,
            lobby,
        }
    }

    pub fn send_input(&self, input: Input) {
        let msg = NetworkActorMessage::SendInput(input);
        self.sender.try_send(msg).expect("Failed to send input");
    }

    pub fn create_lobby(&self, name: String) {
        let msg = NetworkActorMessage::CreateLobby { name };
        self.sender.try_send(msg).expect("Failed to create lobby");
    }

    pub fn join_lobby(&self, name: String) {
        let msg = NetworkActorMessage::JoinLobby { name };
        self.sender.try_send(msg).expect("Failed to join lobby");
    }

    pub fn is_connected(&self) -> bool {
        self.ready.borrow().clone()
    }

    pub fn is_lobby_joined(&self) -> Option<String> {
        self.lobby.borrow().clone()
    }
}
