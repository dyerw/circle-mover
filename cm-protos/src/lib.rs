pub mod cm_proto {
    pub mod messages {
        include!(concat!(env!("OUT_DIR"), "/cm.messages.rs"));
    }
}

use prost::Message;
use std::io::Cursor;

use cm_proto::messages::{CircleMoverMessage, Hello};

pub fn create_hello(name: String) -> Hello {
    let mut hello = Hello::default();
    hello.name = name;
    hello
}

pub fn deserialize_message(buf: &[u8]) -> Result<CircleMoverMessage, prost::DecodeError> {
    CircleMoverMessage::decode(&mut Cursor::new(buf))
}
