use std::time::Duration;

use cm_shared_data::{Input, InputType};
use nalgebra::{point, Point2, Vector2};

#[derive(Copy, Clone, Debug)]
pub struct Circle {
    pub player_id: i32,
    pub circle_id: i64, // auto-incrementing
    pub speed: f32,     // map units per second
    pub position: Point2<f32>,
    pub destination: Option<Point2<f32>>,
}

#[derive(Clone, Debug)]
pub struct Game {
    step_dt: Duration,
    pub circles: Vec<Circle>,
}

impl Game {
    pub fn new(step_dt: Duration) -> Game {
        Game {
            step_dt,
            circles: Vec::new(),
        }
    }

    pub fn step(&mut self) {
        self.step_movement();
    }

    pub fn handle_input(&mut self, input: Input) {
        match input.input_type {
            InputType::CreateCircle { x, y } => self.add_circle(point![x, y], input.player_id),
            InputType::SetDestination { circle_id, x, y } => {
                if self.circle_owned_by(circle_id, input.player_id) {
                    self.set_destination(point![x, y], circle_id)
                }
            }
        }
    }

    pub fn add_circle(&mut self, position: Point2<f32>, player_id: i32) {
        self.circles.push(Circle {
            player_id,
            // Will panic if unit count is higher than i64, unlikely
            circle_id: i64::try_from(self.circles.len()).unwrap(),
            speed: 20.0,
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
    fn step_movement(&mut self) {
        for c in self.circles.iter_mut() {
            if let Some(d) = c.destination {
                let translation_vec: Vector2<f32> = (d - c.position)
                    .normalize()
                    .scale(c.speed * self.step_dt.as_secs_f32());
                let new_pos = c.position + translation_vec;
                println!(
                    "pos: {:?}, translation vec: {:?}, new_pos: {:?}",
                    c.position, translation_vec, new_pos
                );
                c.position = new_pos;
            }
        }
    }

    pub fn circle_owned_by(&self, circle_id: i64, player_id: i32) -> bool {
        match self.circles.iter().find(|c| c.circle_id == circle_id) {
            Some(c) => c.player_id == player_id,
            None => false,
        }
    }
}
