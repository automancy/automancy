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
                let rot = Quaternion::from_rotation_x(-0.4);

                Matrix4::perspective_lh(FRAC_PI_4, 1.0, z_near() as Float, z_far() as Float)
                    * Matrix4::look_to_rh(
                        rot * Vec3::new(0.0, 0.15, 2.85),
                        rot * Vec3::new(0.0, 0.0, 1.0),
                        Vec3::new(0.0, 1.0, 0.0),
                    )
            }
        }
    }
}
