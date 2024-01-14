pub mod cm_proto {
    pub mod messages {
        include!(concat!(env!("OUT_DIR"), "/cm.messages.rs"));
    }
}

use anyhow::Result;
use cm_sim::Input;
use prost::Message;
use prost_types::Timestamp;
use std::{io::Cursor, time::SystemTime};
use tracing::{trace, trace_span};

use cm_proto::messages::{
    circle_mover_message::SubMessage, input_message::InputType, lobby_message::LobbySubMessage,
    CircleMoverMessage, CreateCircle, CreateLobby, InputMessage, JoinLobby, LobbyJoined,
    LobbyMessage, RequestStartGame, SetDestination, SynchronizedGameStart, Vec2,
};

pub fn create_input_message(input: Input) -> CircleMoverMessage {
    let input_type = match input.input_type {
        cm_sim::InputType::CreateCircle { x, y } => InputType::CreateCircle(CreateCircle {
            position: Some(Vec2 { x, y }),
        }),
        cm_sim::InputType::SetDestination { circle_id, x, y } => {
            InputType::SetDestination(SetDestination {
                circle_id,
                position: Some(Vec2 { x, y }),
            })
        }
    };
    let player_input = InputMessage {
        for_tick: input.for_tick,
        player_id: input.player_id,
        input_type: Some(input_type),
    };
    CircleMoverMessage {
        sub_message: Some(SubMessage::InputMessage(player_input)),
    }
}

pub fn create_create_lobby(name: String) -> CircleMoverMessage {
    CircleMoverMessage {
        sub_message: Some(SubMessage::LobbyMessage(LobbyMessage {
            lobby_sub_message: Some(LobbySubMessage::CreateLobby(CreateLobby { name })),
        })),
    }
}

pub fn create_join_lobby(name: String) -> CircleMoverMessage {
    CircleMoverMessage {
        sub_message: Some(SubMessage::LobbyMessage(LobbyMessage {
            lobby_sub_message: Some(LobbySubMessage::JoinLobby(JoinLobby { name })),
        })),
    }
}

pub fn create_lobby_joined(name: String) -> CircleMoverMessage {
    CircleMoverMessage {
        sub_message: Some(SubMessage::LobbyMessage(LobbyMessage {
            lobby_sub_message: Some(LobbySubMessage::LobbyJoined(LobbyJoined { name })),
        })),
    }
}

fn system_time_to_timestamp(ts: SystemTime) -> Timestamp {
    let duration_since_epoch = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let seconds = duration_since_epoch.as_secs() as i64;
    let nanos = duration_since_epoch.subsec_nanos() as i32;

    Timestamp { seconds, nanos }
}

pub fn create_synchronized_game_start(start_at: SystemTime) -> CircleMoverMessage {
    CircleMoverMessage {
        sub_message: Some(SubMessage::LobbyMessage(LobbyMessage {
            lobby_sub_message: Some(LobbySubMessage::SynchronizedGameStart(
                SynchronizedGameStart {
                    start_at: Some(system_time_to_timestamp(start_at)),
                },
            )),
        })),
    }
}

pub fn create_create_request_start_game() -> CircleMoverMessage {
    CircleMoverMessage {
        sub_message: Some(SubMessage::LobbyMessage(LobbyMessage {
            lobby_sub_message: Some(LobbySubMessage::RequestStartGame(RequestStartGame {})),
        })),
    }
}

pub fn serialize_message(msg: CircleMoverMessage) -> Vec<u8> {
    msg.encode_to_vec()
}

pub fn deserialize_message(buf: &[u8]) -> Result<CircleMoverMessage, prost::DecodeError> {
    CircleMoverMessage::decode(&mut Cursor::new(buf))
}

const MAX_MESSAGE_SIZE: usize = 1024;
pub async fn read_message(conn: &quinn::Connection) -> Result<CircleMoverMessage> {
    let span = trace_span!("Reading message");
    let _enter = span.enter();

    let mut recv = conn.accept_uni().await?;
    let bytes = recv.read_to_end(MAX_MESSAGE_SIZE).await?;

    match deserialize_message(&bytes) {
        Ok(msg) => {
            trace!("Successfully deserialized message");
            return Ok(msg);
        }
        Err(e) => {
            return Err(e.into());
        }
    }
}
