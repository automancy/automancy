use automancy_data::{
    math::{Matrix4, Quat, Vec3, consts},
    rendering,
};
use serde::{Deserialize, Serialize};

pub mod audio;
pub mod category;
pub mod font;
pub mod item;
pub mod model;
pub mod recipe;
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
            IconMode::Item => Matrix4::look_at_lh(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            IconMode::Tile => {
                let rot = Quat::rotation_x(-0.4);
                let eye = rot * Vec3::new(0.0, 0.15, 2.85);

                rendering::view::perspective_rh_oz(consts::FRAC_PI_4, 1.0, rendering::view::z_near())
                    * Matrix4::look_at_rh(eye, eye + rot * Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0))
            }
        }
    }
}
