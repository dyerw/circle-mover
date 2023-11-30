use nalgebra::{Point2, Vector2};

#[derive(Copy, Clone, Debug)]
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

    pub fn circle_owned_by(&self, circle_id: i64, player_id: u8) -> bool {
        match self.circles.iter().find(|c| c.circle_id == circle_id) {
            Some(c) => c.player_id == player_id,
            None => false,
        }
    }
}
