use crate::game::data::pos::Pos;

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

pub type Aabb2 = collision::Aabb2<Num>;
pub type Aabb3 = collision::Aabb3<Num>;

pub type Ray3 = collision::Ray3<Num>;

pub type Quaternion = cgmath::Quaternion<Num>;

pub const RENDER_CENTER: Point3 = Point3::new(0.0, 0.0, 0.0);
pub const CHUNK_CENTER: Pos = Pos(0, 0);
