use cgmath::point2;

use crate::game::data::tile::{TileCoord, TileUnit};

use super::hex::offset::OffsetCoord;

pub type Num = f32;

pub type Rad = cgmath::Rad<Num>;
pub fn rad(n: Num) -> Rad {
    cgmath::Rad(n)
}

pub type Deg = cgmath::Deg<Num>;
pub fn deg(n: Num) -> Deg {
    cgmath::Deg(n)
}

pub type Point1 = cgmath::Point1<Num>;
pub type Point2 = cgmath::Point2<Num>;
pub type Point3 = cgmath::Point3<Num>;

pub type Vector1 = cgmath::Vector1<Num>;
pub type Vector2 = cgmath::Vector2<Num>;
pub type Vector3 = cgmath::Vector3<Num>;
pub type Vector4 = cgmath::Vector4<Num>;

pub type Matrix2 = cgmath::Matrix2<Num>;
pub type Matrix3 = cgmath::Matrix3<Num>;
pub type Matrix4 = cgmath::Matrix4<Num>;

pub type Quaternion = cgmath::Quaternion<Num>;

pub type DisplayCoord = OffsetCoord<Num>;

impl DisplayCoord {
    pub const SQRT_3_OVER_TWO: Num = 0.866025403785;
    pub const THREE_OVER_FOUR: Num = 3.0 / 4.0;

    pub fn to_point2(&self) -> Point2 {
        point2(
            self.x() * Self::SQRT_3_OVER_TWO,
            self.y() * Self::THREE_OVER_FOUR,
        )
    }

    pub fn from_point2(point: Point2) -> Self {
        Self::new(
            point.x / Self::SQRT_3_OVER_TWO,
            point.y / Self::THREE_OVER_FOUR,
        )
    }

    pub fn to_tile_coord(&self) -> TileCoord {
        let pos = self.to_point2();

        OffsetCoord::<TileUnit>::new(pos.x as TileUnit, pos.y as TileUnit).to_cube_as_pointy_top()
    }
}
