use std::time::{Duration, Instant};

use nalgebra::{point, Point2, Vector2};
use smol::{
    channel,
    channel::{Receiver, Sender},
    spawn, Task, Timer,
};

#[derive(Clone, Debug)]
pub struct Circle {
    pub player_id: u8,
    pub circle_id: i64, // auto-incrementing
    pub speed: f32,     // map units per second
    pub position: Point2<f32>,
    pub destination: Option<Point2<f32>>,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub circles: Vec<Circle>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Game {
        Game {
            circles: Vec::new(),
        }
    }

    pub fn add_circle(&mut self, position: Point2<f32>, player_id: u8) {
        self.circles.push(Circle {
            player_id,
            // Will panic if unit count is higher than i64, unlikely
            circle_id: i64::try_from(self.circles.len()).unwrap(),
            speed: 1.0,
            position,
            destination: None,
        })
    }
    pub fn set_destination(&mut self, destination: Point2<f32>, circle_id: i64) {
        let circle = self.circles.iter_mut().find(|c| c.circle_id == circle_id);
        if let Some(c) = circle {
            c.destination = Some(destination)
        }
    }
    pub fn step(&mut self, ds: f32) {
        self.step_movement(ds);
    }
    fn step_movement(&mut self, ds: f32) {
        for c in self.circles.iter_mut() {
            if let Some(d) = c.destination {
                let translation_vec: Vector2<f32> =
                    (d - c.position).normalize().scale(c.speed * ds);
                let new_pos = c.position + translation_vec;
                println!(
                    "pos: {:?}, translation vec: {:?}, new_pos: {:?}",
                    c.position, translation_vec, new_pos
                );
                c.position = new_pos;
            }
        }
    }

    fn circle_owned_by(&self, circle_id: i64, player_id: u8) -> bool {
        match self.circles.iter().find(|c| c.circle_id == circle_id) {
            Some(c) => c.player_id == player_id,
            None => false,
        }
    }
}

pub enum InputType {
    CreateCircle { x: f32, y: f32 },
    SetDestination { circle_id: i64, x: f32, y: f32 },
}

pub struct Input {
    pub player_id: u8,
    pub input_type: InputType,
}

pub struct CmSim {
    game: Game,
    current_tick: u128,
}

impl Default for CmSim {
    fn default() -> Self {
        Self::new()
    }
}

impl CmSim {
    pub fn new() -> CmSim {
        CmSim {
            game: Game::new(),
            current_tick: 0,
        }
    }

    pub fn start(
        tick_length: Duration,
    ) -> (Task<()>, oneshot::Sender<()>, Receiver<Game>, Sender<Input>) {
        let (stop_sender, stop_receiver) = oneshot::channel::<()>();
        let (state_sender, state_receiver) = channel::unbounded::<Game>();
        let (input_sender, input_receiver) = channel::unbounded::<Input>();

        let mut cm_sim = CmSim::new();
        let task = spawn(async move {
            loop {
                let now = Instant::now();
                cm_sim.tick(tick_length.as_secs_f32() / 1.0, &input_receiver);
                let elapsed = now.elapsed();

                // Slow down to match tick rate
                if elapsed < tick_length {
                    Timer::after(tick_length - elapsed).await;
                } else {
                    // Uh oh! Server isn't able to keep up with tick rate, what do?
                }

                cm_sim.current_tick += 1;
                match state_sender.send(cm_sim.game.clone()).await {
                    Ok(()) => {}
                    Err(e) => {
                        println!("{}", e)
                    }
                }

                println!(
                    "tick: {} took {}µs",
                    cm_sim.current_tick,
                    elapsed.as_micros()
                );
                if let Ok(()) = stop_receiver.try_recv() {
                    println!("Stop received");
                    // Stop loop, finish task
                    break;
                }
            }
        });
        (task, stop_sender, state_receiver, input_sender)
    }

    fn tick(&mut self, ds: f32, input_receiver: &Receiver<Input>) {
        let mut n = 0;
        while !input_receiver.is_empty() {
            n += 1;
            let input = input_receiver.try_recv().unwrap();
            match input.input_type {
                InputType::CreateCircle { x, y } => {
                    self.game.add_circle(point![x, y], input.player_id)
                }
                InputType::SetDestination { circle_id, x, y } => {
                    if self.game.circle_owned_by(circle_id, input.player_id) {
                        self.game.set_destination(point![x, y], circle_id)
                    }
                }
            }
        }
        println!("Received {} inputs this tick", n);
        self.game.step(ds);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_stops() {
        smol::block_on(async {
            let (task, stop_chan, _state_rec, _) = CmSim::start(Duration::from_millis(250));
            let _ = stop_chan.send(());
            task.await
        })
    }

    #[test]
    fn it_adds_circles() {
        smol::block_on(async {
            let (task, stop_chan, state_rec, input_sender) =
                CmSim::start(Duration::from_millis(250));

            for _ in 0..100 {
                Timer::after(Duration::from_millis(250)).await;
                let _ = input_sender
                    .send(Input {
                        player_id: 0,
                        input_type: InputType::CreateCircle { x: 0.0, y: 0.0 },
                    })
                    .await;
            }

            Timer::after(Duration::from_secs(2)).await;
            let _ = stop_chan.send(());

            let mut states: Vec<Game> = vec![];

            while !state_rec.is_empty() {
                let state = state_rec.try_recv().unwrap();
                states.push(state);
            }

            println!("{:?}", states.len());
            task.await
        })
    }

    #[test]
    fn circles_move() {
        smol::block_on(async {
            let (task, stop_chan, state_rec, input_sender) =
                CmSim::start(Duration::from_millis(250));

            let _ = input_sender
                .send(Input {
                    player_id: 0,
                    input_type: InputType::CreateCircle { x: 0.0, y: 0.0 },
                })
                .await;

            let _ = input_sender
                .send(Input {
                    player_id: 0,
                    input_type: InputType::SetDestination {
                        circle_id: 0,
                        x: 5.0,
                        y: 0.0,
                    },
                })
                .await;

            Timer::after(Duration::from_secs(5)).await;
            let _ = stop_chan.send(());

            let mut states: Vec<Game> = vec![];

            while !state_rec.is_empty() {
                let state = state_rec.try_recv().unwrap();
                states.push(state);
            }

            println!("{:?}", states.last());
            task.await
        })
    }
}
