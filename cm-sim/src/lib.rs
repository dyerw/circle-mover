pub mod actor;
pub mod game;
mod tick_sequenced_stream;

#[derive(Debug, Copy, Clone)]
pub enum InputType {
    CreateCircle { x: f32, y: f32 },
    SetDestination { circle_id: i64, x: f32, y: f32 },
}

#[derive(Debug, Copy, Clone)]
pub struct Input {
    pub for_tick: i32,
    pub player_id: i32,
    pub input_type: InputType,
}
