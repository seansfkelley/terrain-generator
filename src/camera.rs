use std::f32::consts::{ PI, FRAC_PI_2,  };
use glm;

const TWO_PI: f32 = PI * 2.0;

#[derive(Debug)]
pub struct Camera {
    pub pos: glm::Vec3,
    azimuth: f32,
    inclination: f32,
    pub field_of_view: f32,
}

pub enum TranslateDirection {
    Forward,
    Side,
    Altitude,
}

pub enum LookDirection {
    Vertical,
    Horizontal,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            pos: glm::vec3(0.0, 0.0, 1.0),
            azimuth: PI, // Look at -Z (into the screen).
            inclination: FRAC_PI_2, // Look at the horizon.
            field_of_view: 45.0,
        }
    }

    pub fn look(&mut self, dir: LookDirection, amount: f32) {
        match dir {
            LookDirection::Vertical   => {
                self.inclination = (self.inclination - amount).max(0.01).min(PI - 0.01);
            },
            LookDirection::Horizontal => {
                self.azimuth = ((self.azimuth + amount) + TWO_PI) % TWO_PI;
            },
        }
    }

    pub fn direction(&self) -> glm::Vec3 {
        let reverse_inclination = FRAC_PI_2 - self.inclination;
        glm::vec3(
            reverse_inclination.cos() * self.azimuth.sin(),
            reverse_inclination.sin(),
            reverse_inclination.cos() * self.azimuth.cos(),
        )
    }

    fn right(&self) -> glm::Vec3 {
        let rotated_azimuth = self.azimuth - FRAC_PI_2;
        glm::vec3(
            rotated_azimuth.sin(),
            0.0,
            rotated_azimuth.cos(),
        )
    }

    pub fn up(&self) -> glm::Vec3 {
        glm::cross(self.right(), self.direction())
    }

    pub fn translate(&mut self, dir: TranslateDirection, amount: f32) {
        match dir {
            TranslateDirection::Forward  => {
                self.pos = self.pos + self.direction() * amount;
            },
            TranslateDirection::Side     => {
                self.pos = self.pos + self.right() * amount;
            },
            TranslateDirection::Altitude => {
                self.pos = self.pos + self.up() * amount;
            },
        }
    }
}
