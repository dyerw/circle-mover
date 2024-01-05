pub mod cm_proto {
    pub mod messages {
        include!(concat!(env!("OUT_DIR"), "/cm.messages.rs"));
    }
}

use prost::Message;
use std::io::Cursor;

use cm_proto::messages::{circle_mover_message::Value, CircleMoverMessage, Goodbye, Hello};

pub fn create_hello(name: String) -> Hello {
    let mut hello = Hello::default();
    hello.name = name;
    hello
}

pub fn create_goodbye(name: String) -> Goodbye {
    let mut goodbye = Goodbye::default();
    goodbye.name = name;
    goodbye
}

pub fn create_message(v: Value) -> CircleMoverMessage {
    let mut msg = CircleMoverMessage::default();
    msg.value = Some(v);
    msg
}

pub fn serialize_message(msg: CircleMoverMessage) -> Vec<u8> {
    let encoded_vec = msg.encode_length_delimited_to_vec();
    encoded_vec
}

pub fn deserialize_message(buf: &[u8]) -> Result<CircleMoverMessage, prost::DecodeError> {
    // CircleMoverMessage::decode_length_delimited(buf)
    CircleMoverMessage::decode(&mut Cursor::new(buf))
}
