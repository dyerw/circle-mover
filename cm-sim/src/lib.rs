use std::time::{Duration, Instant};

use nalgebra::{Point2, Vector2};
use smol::{
    channel,
    channel::{Receiver, Sender},
    spawn, Task, Timer,
};

#[derive(Clone, Debug)]
pub struct Circle {
    _player_id: u8,
    circle_id: u128, // auto-incrementing
    speed: f32,      // map units per second
    position: Point2<f32>,
    destination: Option<Point2<f32>>,
}

#[derive(Clone, Debug)]
pub struct Game {
    circles: Vec<Circle>,
}

impl Game {
    pub fn new() -> Game {
        Game {
            circles: Vec::new(),
        }
    }

    pub fn add_circle(&mut self, position: Point2<f32>, player_id: u8) {
        self.circles.push(Circle {
            _player_id: player_id,
            // Will panic if unit count is higher than u128, unlikely
            circle_id: u128::try_from(self.circles.len()).unwrap(),
            speed: 1.0,
            position,
            destination: None,
        })
    }
    pub fn set_destination(&mut self, destination: Point2<f32>, circle_id: u128) {
        let circle = self.circles.iter_mut().find(|c| c.circle_id == circle_id);
        match circle {
            Some(c) => c.destination = Some(destination),
            None => {}
        }
    }
    pub fn step(&mut self, ds: f32) {
        self.step_movement(ds);
    }
    fn step_movement(&mut self, ds: f32) {
        for c in self.circles.iter_mut() {
            match c.destination {
                Some(d) => {
                    let translation_vec: Vector2<f32> =
                        (c.position - d).normalize().scale(c.speed * ds);
                    c.position = c.position + translation_vec;
                }
                None => {}
            }
        }
    }
}

pub enum InputType {
    CreateCircle(Point2<f32>),
    SetDestination(u128, Point2<f32>),
}

pub struct Input {
    player_id: u8,
    input_type: InputType,
}

pub struct CmSim {
    game: Game,
    current_tick: u128,
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
                match stop_receiver.try_recv() {
                    Ok(()) => {
                        println!("Stop received");
                        // Stop loop, finish task
                        break;
                    }
                    Err(_) => {}
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
                InputType::CreateCircle(p) => self.game.add_circle(p, input.player_id),
                InputType::SetDestination(id, d) => self.game.set_destination(d, id),
            }
        }
        println!("Received {} inputs this tick", n);
        self.game.step(ds);
    }
}

#[cfg(test)]
mod tests {

    use nalgebra::point;

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
                        input_type: InputType::CreateCircle(point![0.0, 0.0]),
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

            println!("{:?}", states);
            task.await
        })
    }
}
