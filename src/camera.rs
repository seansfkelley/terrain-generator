use std::f32::consts::{ PI, FRAC_PI_2,  };
use glm;

// static AZIMUTH_REF_DIR: glm::Vec3 = glm::vec3(0.0, 0.0, -1.0);
// static ZENITH: glm::Vec3 = glm::vec3(0.0, 1.0, 0.0);

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
            pos: glm::vec3(4.0, 3.0, 3.0),
            // azimuth: PI,
            azimuth: -2.0,
            // inclination: FRAC_PI_2,
            inclination: 2.2,
            field_of_view: 45.0,
        }
    }

    pub fn look(&mut self, dir: LookDirection, amount: f32) {
        match dir {
            LookDirection::Vertical   => {
                self.inclination -= amount;
                // TODO: Make sure it can't go beyond +/- 90.
            },
            LookDirection::Horizontal => {
                // TODO: clamp
                self.azimuth = clamp(self.azimuth + amount, 0, TWO_PI);
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
        glm::vec3(
            self.azimuth.sin(),
            0.0,
            self.azimuth.cos(),
        )
    }

    pub fn up(&self) -> glm::Vec3 {
        self.right() * self.direction()
    }

    // pub fn translate(&mut self, dir: TranslateDirection, amount: f32) {
    //     match dir {
    //         TranslateDirection::Forward  => {
    //             self.pos = self.pos + self.dir * amount;
    //         },
    //         TranslateDirection::Side     => {
    //             // Literally no idea if this is correct.
    //             self.pos = self.pos + (self.dir * self.up) * amount;
    //         },
    //         TranslateDirection::Altitude => {
    //             self.pos = self.pos + self.up * amount;
    //         },
    //     }
    // }
}
