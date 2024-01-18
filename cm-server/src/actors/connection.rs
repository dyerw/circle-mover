use std::time::SystemTime;

use anyhow::Result;
use cm_shared_data::{
    read_message, ClientLobbyMessage, ClientNetworkMessage, ServerNetworkMessage,
};
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::info;

use super::{lobby::LobbyMessage, server::ServerMessage};

pub enum ConnectionMessage {
    ReceivedNetworkMessage(ClientNetworkMessage),
    JoinedLobby {
        name: String,
        lobby_ref: ActorRef<LobbyMessage>,
    },
    SendSynchronizedGameStart(SystemTime),
    LostConnection,
}

pub struct ConnectionState {
    connection: quinn::Connection,
    server_ref: ActorRef<ServerMessage>,
    lobby_ref: Option<ActorRef<LobbyMessage>>,
}

pub struct ConnectionArguments {
    pub connecting: quinn::Connecting,
    pub server_ref: ActorRef<ServerMessage>,
}

pub struct ConnectionActor;

#[async_trait]
impl Actor for ConnectionActor {
    type State = ConnectionState;
    type Msg = ConnectionMessage;
    type Arguments = ConnectionArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("Client connecting");
        let connection = arguments.connecting.await?;

        // Kick off task to poll the connection for incoming streams from client
        let connection_clone = connection.clone();
        tokio::spawn(async move {
            loop {
                match read_message(&connection_clone).await.and_then(|msg| {
                    myself.cast(ConnectionMessage::ReceivedNetworkMessage(msg))?;
                    Ok(())
                }) {
                    Ok(()) => {}
                    Err(_) => {
                        info!("Received error reading message, losing connection");
                        myself
                            .cast(ConnectionMessage::LostConnection)
                            .expect("Failed to send lost connection");
                        break;
                    }
                }
            }
            // info!("Ending async task for reading from connection {}", connection_clone.);
        });

        info!("Client connected");
        Ok(ConnectionState {
            connection,
            lobby_ref: None,
            server_ref: arguments.server_ref,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ConnectionMessage::ReceivedNetworkMessage(msg) => {
                info!("Received message: {:?}", msg);
                match msg {
                    ClientNetworkMessage::LobbyMessage(ClientLobbyMessage::RequestStartGame) => {
                        if let Some(ref lobby) = state.lobby_ref {
                            lobby.cast(LobbyMessage::RequestStartGame)?;
                        }
                    }
                    ClientNetworkMessage::LobbyMessage(ClientLobbyMessage::CreateLobby {
                        name,
                    }) => {
                        state
                            .server_ref
                            .cast(ServerMessage::CreateLobby { name, host: myself })?;
                    }
                    _ => {}
                };
            }
            ConnectionMessage::JoinedLobby { name, lobby_ref } => {
                info!("Joined lobby {}", name);
                state.lobby_ref = Some(lobby_ref);
                let mut send = state.connection.open_uni().await?;
                // FIXME: handle other players
                let bytes = ServerNetworkMessage::lobby_joined(name, vec![])?;
                send.write_all(&bytes).await?;
                send.finish().await?;
            }
            ConnectionMessage::SendSynchronizedGameStart(start_at) => {
                let mut send = state.connection.open_uni().await?;
                let bytes = ServerNetworkMessage::synchronized_game_start(start_at)?;
                send.write_all(&bytes).await?;
                send.finish().await?;
            }
            ConnectionMessage::LostConnection => {
                info!("Connection lost");
                if let Some(l) = &state.lobby_ref {
                    l.cast(LobbyMessage::LostConnection(myself.get_id()))?;
                }
                state
                    .server_ref
                    .cast(ServerMessage::LostConnection(myself.get_id()))?;
                myself.stop(Some("Connection lost".to_string()))
            }
        }
        Ok(())
    }
}
