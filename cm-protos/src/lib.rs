pub mod cm_proto {
    pub mod messages {
        include!(concat!(env!("OUT_DIR"), "/cm.messages.rs"));
    }
}

use cm_sim::Input;
use prost::Message;
use std::io::Cursor;

use cm_proto::messages::{
    circle_mover_message::SubMessage, input_message::InputType, lobby_message::LobbySubMessage,
    CircleMoverMessage, CreateCircle, CreateLobby, InputMessage, JoinLobby, LobbyMessage,
    SetDestination, Vec2,
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

pub fn serialize_message(msg: CircleMoverMessage) -> Vec<u8> {
    msg.encode_to_vec()
}

pub fn deserialize_message(buf: &[u8]) -> Result<CircleMoverMessage, prost::DecodeError> {
    CircleMoverMessage::decode(&mut Cursor::new(buf))
}
