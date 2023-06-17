#![allow(unused_qualifications)]

use cgmath::BaseFloat;

pub type Float = f32;

pub type Rad = cgmath::Rad<Float>;

pub fn rad(n: Float) -> Rad {
    cgmath::Rad(n)
}

pub type Deg = cgmath::Deg<Float>;

pub fn deg(n: Float) -> Deg {
    cgmath::Deg(n)
}

pub type Point1 = cgmath::Point1<Float>;
pub type Point2 = cgmath::Point2<Float>;
pub type Point3 = cgmath::Point3<Float>;

pub type Vector1 = cgmath::Vector1<Float>;
pub type Vector2 = cgmath::Vector2<Float>;
pub type Vector3 = cgmath::Vector3<Float>;
pub type Vector4 = cgmath::Vector4<Float>;

pub type Matrix2 = cgmath::Matrix2<Float>;
pub type Matrix3 = cgmath::Matrix3<Float>;
pub type Matrix4 = cgmath::Matrix4<Float>;

pub type Quaternion = cgmath::Quaternion<Float>;

pub type Double = f64;

pub type DPoint1 = cgmath::Point1<Double>;
pub type DPoint2 = cgmath::Point2<Double>;
pub type DPoint3 = cgmath::Point3<Double>;

pub type DVector1 = cgmath::Vector1<Double>;
pub type DVector2 = cgmath::Vector2<Double>;
pub type DVector3 = cgmath::Vector3<Double>;
pub type DVector4 = cgmath::Vector4<Double>;

pub type DMatrix2 = cgmath::Matrix2<Double>;
pub type DMatrix3 = cgmath::Matrix3<Double>;
pub type DMatrix4 = cgmath::Matrix4<Double>;

pub type DQuaternion = cgmath::Quaternion<Double>;

#[rustfmt::skip]
pub fn perspective<N: BaseFloat>(fov_y: N, a: N, n: N, f: N) -> cgmath::Matrix4<N> {
    let zero = N::zero();
    let one = N::one();
    let two = one + one;

    let t = fov_y.div(two).tan();
    let d = f - n;
    let m = -(f * n);

    cgmath::Matrix4::<N>::new(
        one / (t * a), zero, zero, zero,
        zero, one / t, zero, zero,
        zero, zero, f / d, one,
        zero, zero, m / d, zero,
    )
}

pub fn matrix<N: BaseFloat>(pos: cgmath::Point3<N>, aspect: N, pi: N) -> cgmath::Matrix4<N> {
    let view = view(pos);
    let projection = projection(aspect, pi);

    projection * view
}

pub fn view<N: BaseFloat>(pos: cgmath::Point3<N>) -> cgmath::Matrix4<N> {
    cgmath::Matrix4::<N>::look_to_rh(
        pos,
        cgmath::Vector3::<N>::unit_z(),
        cgmath::Vector3::<N>::unit_y(),
    )
}

pub fn projection<N: BaseFloat>(aspect: N, pi: N) -> cgmath::Matrix4<N> {
    let one = N::one();
    let two = one + one;
    let ten = two + two + two + two + two;

    perspective(pi / two, aspect, one / ten.powi(2), ten.powi(4))
}
