#![allow(unused_qualifications)]

use std::f64::consts::PI;
use std::ops::{Div, Sub};

use cgmath::{point2, point3, vec2, Angle, BaseFloat, EuclideanSpace};
use hexagon_tiles::fractional::FractionalHex;
use hexagon_tiles::layout::{Layout, LAYOUT_ORIENTATION_POINTY};
use hexagon_tiles::point::Point;
use hexagon_tiles::traits::HexRound;

use crate::coord::{TileCoord, TileHex, TileRange};

const HEX_GRID_LAYOUT: Layout = Layout {
    orientation: LAYOUT_ORIENTATION_POINTY,
    size: Point { x: 1.0, y: 1.0 },
    origin: Point { x: 0.0, y: 0.0 },
};

pub const FAR: Double = 0.0;

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
    let projection = projection(aspect, pi);
    let view = view(pos);

    projection * view
}

pub fn view<N: BaseFloat>(pos: cgmath::Point3<N>) -> cgmath::Matrix4<N> {
    cgmath::Matrix4::<N>::look_to_rh(
        pos,
        cgmath::Vector3::<N>::unit_z(),
        cgmath::Vector3::<N>::unit_y(),
    )
}

pub fn z_near<N: BaseFloat>() -> N {
    let one = N::one();
    let two = one + one;
    let ten = two + two + two + two + two;

    one / ten.powi(2)
}

pub fn z_far<N: BaseFloat>() -> N {
    let one = N::one();
    let two = one + one;
    let ten = two + two + two + two + two;

    ten.powi(4)
}

pub fn projection<N: BaseFloat>(aspect: N, pi: N) -> cgmath::Matrix4<N> {
    let one = N::one();
    let two = one + one;

    perspective(pi / two, aspect, z_near(), z_far())
}

pub fn pixel_to_hex<N: BaseFloat>(p: cgmath::Point2<N>) -> FractionalHex<Double> {
    hexagon_tiles::layout::pixel_to_hex(
        HEX_GRID_LAYOUT,
        hexagon_tiles::point::point(p.x.to_f64().unwrap(), p.y.to_f64().unwrap()),
    )
}

pub fn hex_to_pixel(hex: TileHex) -> DPoint2 {
    let p = hexagon_tiles::layout::hex_to_pixel(HEX_GRID_LAYOUT, hex);

    point2(p.x, p.y)
}

pub fn frac_hex_to_pixel(hex: FractionalHex<Double>) -> DPoint2 {
    let p = hexagon_tiles::layout::frac_hex_to_pixel(HEX_GRID_LAYOUT, hex);

    point2(p.x, p.y)
}

/// Gets the hex position being pointed at.
#[inline]
pub fn main_pos_to_hex(
    (width, height): (Double, Double),
    camera_pos: DPoint3,
    main_pos: DPoint2,
) -> FractionalHex<Double> {
    let p = screen_to_world((width, height), main_pos, camera_pos);

    pixel_to_hex(point2(p.x, p.y))
}

/// Converts screen space coordinates into normalized coordinates.
#[inline]
pub fn screen_to_normalized((width, height): (Double, Double), c: DPoint2) -> DPoint3 {
    let size = vec2(width, height) * 0.5;

    let c = vec2(c.x, c.y);
    let c = c.zip(size, Sub::sub);
    let c = c.zip(size, Div::div);

    point3(c.x, c.y, 0.0)
}

/// Converts screen coordinates to world coordinates.
#[inline]
pub fn screen_to_world(
    (width, height): (Double, Double),
    pos: DPoint2,
    camera_pos: DPoint3,
) -> DPoint3 {
    let pos = screen_to_normalized((width, height), pos);

    normalized_to_world((width, height), point2(pos.x, pos.y), camera_pos)
}

/// Converts normalized screen coordinates to world coordinates.
#[inline]
pub fn normalized_to_world(
    (width, height): (Double, Double),
    pos: DPoint2,
    camera_pos: DPoint3,
) -> DPoint3 {
    let aspect = width / height;
    let aspect_squared = aspect * aspect;

    let matrix = matrix(point3(0.0, 0.0, camera_pos.z), aspect, PI);

    let pos = pos.to_vec();
    let pos = matrix * pos.extend(FAR).extend(1.0);
    let pos = pos.truncate() * pos.w;

    point3(pos.x * aspect_squared, pos.y, pos.z) + camera_pos.to_vec()
}

/// Converts hex coordinates to normalized screen coordinates.
#[inline]
pub fn hex_to_normalized(
    (width, height): (Double, Double),
    camera_pos: DPoint3,
    hex: TileCoord,
) -> DPoint3 {
    let p = hex_to_pixel(hex.into()).to_vec();

    let aspect = width / height;

    let matrix = matrix(camera_pos, aspect, PI);

    let p = matrix * p.extend(FAR).extend(1.0);
    let w = p.w;
    let p = p.truncate() / w;

    point3(p.x, p.y, p.z)
}

/// Gets the culling range from the camera's position
pub fn get_culling_range((width, height): (Double, Double), camera_pos: DPoint3) -> TileRange {
    let a = normalized_to_world((width, height), point2(-1.0, -1.0), camera_pos);
    let b = normalized_to_world((width, height), point2(1.0, 1.0), camera_pos);

    let a = pixel_to_hex(point2(a.x, a.y)).round().into();
    let b = pixel_to_hex(point2(b.x, b.y)).round().into();

    TileRange::new(a, b).extend(4)
}

pub fn direction_to_angle(d: DVector2) -> Rad {
    let angle = cgmath::Rad::atan2(d.y, d.x);

    rad(angle.0.rem_euclid(PI) as Float)
}
