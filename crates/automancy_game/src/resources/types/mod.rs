use automancy_data::{
    math::{Matrix4, Quat, Vec3, consts},
    rendering::camera,
};
use serde::{Deserialize, Serialize};

pub mod audio;
pub mod category;
pub mod font;
pub mod function;
pub mod item;
pub mod model;
pub mod research;
pub mod script;
pub mod shader;
pub mod tag;
pub mod tile;
pub mod translate;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum IconMode {
    Item,
    Tile,
}

impl IconMode {
    pub fn model_matrix(self) -> Matrix4 {
        match self {
            IconMode::Item => Matrix4::default(),
            IconMode::Tile => Matrix4::default(),
        }
    }
    pub fn world_matrix(self) -> Matrix4 {
        match self {
            IconMode::Item => Matrix4::look_to_rh(
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            IconMode::Tile => {
                let rot = Quat::from_rotation_x(-0.4);
                let eye = rot * Vec3::new(0.0, 0.15, 2.85);

                Matrix4::perspective_lh_zo(
                    consts::FRAC_PI_4,
                    1.0,
                    camera::z_near(),
                    camera::z_far(),
                ) * Matrix4::look_at_lh(
                    eye,
                    eye + rot * Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 1.0, 0.0),
                )
            }
        }
    }
}
