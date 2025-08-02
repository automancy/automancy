pub mod consts {
    pub use core::f32::consts::*;
}

pub type Float = f32;
pub type Int = i32;
pub type UInt = UInt;

pub type Vec2 = vek::Vec2<Float>;
pub type Vec3 = vek::Vec3<Float>;
pub type Vec4 = vek::Vec4<Float>;

pub type IVec2 = vek::Vec2<Int>;
pub type IVec3 = vek::Vec3<Int>;
pub type IVec4 = vek::Vec4<Int>;

pub type UVec2 = vek::Vec2<UInt>;
pub type UVec3 = vek::Vec3<UInt>;
pub type UVec4 = vek::Vec4<UInt>;

pub type Matrix2 = vek::Mat2<Float>;
pub type Matrix3 = vek::Mat3<Float>;
pub type Matrix4 = vek::Mat4<Float>;
pub type Quat = vek::Quaternion<Float>;

#[inline]
#[must_use]
pub fn vec_to_angle(d: Vec2) -> Float {
    let angle = d.y.atan2(d.x);

    angle.rem_euclid(consts::PI)
}
