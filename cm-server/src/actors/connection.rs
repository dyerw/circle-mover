use std::time::SystemTime;

use anyhow::Result;
use cm_protos::{
    cm_proto::messages::{
        circle_mover_message::SubMessage, lobby_message::LobbySubMessage, CircleMoverMessage,
        CreateLobby, LobbyMessage as LM, RequestStartGame,
    },
    create_lobby_joined, create_synchronized_game_start, read_message,
};
use prost::Message;
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::{error, info};

use super::{lobby::LobbyMessage, server::ServerMessage};

pub enum ConnectionMessage {
    ReceivedNetworkMessage(CircleMoverMessage),
    JoinedLobby {
        name: String,
        lobby_ref: ActorRef<LobbyMessage>,
    },
    SendSynchronizedGameStart(SystemTime),
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
                    Err(e) => {
                        error!("Failed: {reason}", reason = e.to_string());
                    }
                }
            }
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
                    CircleMoverMessage {
                        sub_message:
                            Some(SubMessage::LobbyMessage(LM {
                                lobby_sub_message:
                                    Some(LobbySubMessage::RequestStartGame(RequestStartGame {})),
                            })),
                    } => {
                        if let Some(ref lobby) = state.lobby_ref {
                            lobby.cast(LobbyMessage::RequestStartGame)?;
                        }
                    }
                    CircleMoverMessage {
                        sub_message:
                            Some(SubMessage::LobbyMessage(LM {
                                lobby_sub_message:
                                    Some(LobbySubMessage::CreateLobby(CreateLobby { name })),
                            })),
                    } => {
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
                send.write_all(&create_lobby_joined(name).encode_to_vec())
                    .await?;
                send.finish().await?;
            }
            ConnectionMessage::SendSynchronizedGameStart(start_at) => {
                let mut send = state.connection.open_uni().await?;
                send.write_all(&create_synchronized_game_start(start_at).encode_to_vec())
                    .await?;
                send.finish().await?;
            }
        }
        Ok(())
    }
}
