pub mod cm_proto {
    pub mod messages {
        include!(concat!(env!("OUT_DIR"), "/cm.messages.rs"));
    }
}

use cm_proto::messages;

pub fn create_hello(name: String) -> messages::Hello {
    let mut hello = messages::Hello::default();
    hello.name = name;
    hello
}
