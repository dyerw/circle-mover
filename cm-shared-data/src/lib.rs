use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::SystemTime;

/// Messages from server to client
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerNetworkMessage {
    LobbyMessage(ServerLobbyMessage),
    InputMessage(Input),
}

/// Just a bunch of static utility functions for creating serialized message bytes
impl ServerNetworkMessage {
    pub fn lobby_joined(name: String, other_players: Vec<String>) -> Result<Vec<u8>> {
        serialize_server_message(&Self::LobbyMessage(ServerLobbyMessage::LobbyJoined {
            name,
            other_players,
        }))
    }

    pub fn synchronized_game_start(start_at: SystemTime) -> Result<Vec<u8>> {
        serialize_server_message(&Self::LobbyMessage(
            ServerLobbyMessage::SynchronizedGameStart { start_at },
        ))
    }
}

/// Messages from client to server
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientNetworkMessage {
    LobbyMessage(ClientLobbyMessage),
    InputMessage(Input),
}

impl ClientNetworkMessage {
    pub fn create_lobby(name: String) -> Result<Vec<u8>> {
        serialize_client_message(&Self::LobbyMessage(ClientLobbyMessage::CreateLobby {
            name,
        }))
    }

    pub fn join_lobby(name: String) -> Result<Vec<u8>> {
        serialize_client_message(&Self::LobbyMessage(ClientLobbyMessage::JoinLobby { name }))
    }

    pub fn request_start_game() -> Result<Vec<u8>> {
        serialize_client_message(&Self::LobbyMessage(ClientLobbyMessage::RequestStartGame))
    }

    pub fn input(input: Input) -> Result<Vec<u8>> {
        serialize_client_message(&Self::InputMessage(input))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerLobbyMessage {
    LobbyJoined {
        name: String,
        other_players: Vec<String>,
    },
    SynchronizedGameStart {
        start_at: SystemTime,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientLobbyMessage {
    CreateLobby { name: String },
    JoinLobby { name: String },
    RequestStartGame,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum InputType {
    CreateCircle { x: f32, y: f32 },
    SetDestination { circle_id: i64, x: f32, y: f32 },
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Input {
    pub for_tick: i32,
    pub player_id: i32,
    pub input_type: InputType,
}

pub fn serialize_client_message(msg: &ClientNetworkMessage) -> Result<Vec<u8>> {
    let bytes = bincode::serialize(msg)?;
    Ok(bytes)
}

pub fn serialize_server_message(msg: &ServerNetworkMessage) -> Result<Vec<u8>> {
    let bytes = bincode::serialize(msg)?;
    Ok(bytes)
}

// FIXME: Determine the actual max message size once we've figured out what all the messages will be
const MAX_MESSAGE_SIZE: usize = 1024;

#[tracing::instrument]
pub async fn read_message<T: DeserializeOwned>(conn: &quinn::Connection) -> Result<T> {
    let mut recv = conn.accept_uni().await?;
    let bytes = recv.read_to_end(MAX_MESSAGE_SIZE).await?;

    match bincode::deserialize(&bytes[..]) {
        Ok(msg) => {
            tracing::trace!("Successfully deserialized message");
            return Ok(msg);
        }
        Err(e) => {
            return Err(e.into());
        }
    }
}
