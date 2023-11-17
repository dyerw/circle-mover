use std::time::Duration;

use cm_sim::{CmSim, Game};
use godot::prelude::*;
use smol::block_on;

struct CmSimExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CmSimExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct CmSimGD {
    #[base]
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for CmSimGD {
    fn init(base: Base<RefCounted>) -> Self {
        godot_print!("Hello, world!"); // Prints to the Godot console
        Self { base }
    }
}

#[godot_api]
impl CmSimGD {
    #[func]
    fn start_sim(&mut self) {
        godot_print!("Starting sim from rust");
        let (task, stop_chan, state_rec, input_sender) = CmSim::start(Duration::from_millis(250));
        block_on(async {
            let game = state_rec.recv().await;
            godot_print!("{:?}", game)
        })
    }

    #[signal]
    fn sim_state_update();
}
