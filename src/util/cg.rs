#![allow(dead_code)]
#![allow(unused_qualifications)]

use cgmath::BaseFloat;

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
        one / (t * a), zero   , zero , zero,
        zero         , one / t, zero , zero,
        zero         , zero   , f / d, one ,
        zero         , zero   , m / d, zero,
    )
}

pub fn matrix<N: BaseFloat>(pos: cgmath::Point3<N>, aspect: N, pi: N) -> cgmath::Matrix4<N> {
    let view = view(pos, pi);
    let projection = projection(aspect, pi);

    projection * view
}

pub fn eye<N: BaseFloat>(z: N, pi: N) -> cgmath::Vector3<N> {
    let one = N::one();
    let two = one + one;
    let two_point_two_five = two + (one / (two * two));

    let z = one - z.min(one);
    let r = z.mul(pi / two).sin();
    let o = r.mul(pi / two_point_two_five).cos();

    cgmath::vec3(N::zero(), r, o)
}

pub fn actual_pos<N: BaseFloat>(
    pos: cgmath::Point3<N>,
    eye: cgmath::Vector3<N>,
) -> cgmath::Point3<N> {
    let one = N::one();
    let two = one + one;
    let six = two + two + two;

    cgmath::point3(pos.x, pos.y, (pos.z * six) + (one / two) + eye.z)
}

pub fn view<N: BaseFloat>(pos: cgmath::Point3<N>, pi: N) -> cgmath::Matrix4<N> {
    let eye = eye(pos.z, pi);
    let actual_pos = actual_pos(pos, eye);

    cgmath::Matrix4::<N>::look_to_rh(actual_pos, eye, cgmath::Vector3::<N>::unit_y())
}

pub fn projection<N: BaseFloat>(aspect: N, pi: N) -> cgmath::Matrix4<N> {
    let one = N::one();
    let two = one + one;
    let ten = two + two + two + two + two;

    perspective(pi / two, aspect, one / ten.powi(2), ten.powi(4))
}
