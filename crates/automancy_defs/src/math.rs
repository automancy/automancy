use std::f32::consts::PI;

pub use glam::{swizzles::*, vec2, vec3, vec4};
use hexx::{HexLayout, HexOrientation};

use crate::coord::{TileBounds, TileCoord};

pub const HEX_GRID_LAYOUT: HexLayout = HexLayout {
    orientation: HexOrientation::Pointy,
    origin: Vec2::ZERO,
    scale: Vec2::NEG_ONE,
};

pub const SQRT_3: Float = 1.732_050_8;

pub const FAR: Float = 0.0;

pub type Float = f32;

pub type Vec2 = glam::Vec2;
pub type Vec3 = glam::Vec3;
pub type Vec4 = glam::Vec4;

pub type IVec2 = glam::IVec2;
pub type IVec3 = glam::IVec3;
pub type IVec4 = glam::IVec4;

pub type Matrix2 = glam::Mat2;
pub type Matrix3 = glam::Mat3;
pub type Matrix4 = glam::Mat4;

pub type Quaternion = glam::Quat;

#[inline]
pub fn z_near() -> Float {
    0.1
}

#[inline]
pub fn z_far() -> Float {
    100.0
}

#[inline]
pub fn fov() -> Float {
    PI / 2.0
}

pub fn camera_angle(z: Float) -> Float {
    // TODO magic values
    let max = 6.5;

    if z < max {
        let normalized = (max - z) / 4.0;

        normalized / -1.5
    } else {
        0.0
    }
}

fn projection(aspect: Float) -> Matrix4 {
    Matrix4::perspective_lh(fov(), aspect, z_near(), z_far())
}

fn camera_view(pos: Vec3) -> Matrix4 {
    Matrix4::look_to_rh(
        pos,
        Quaternion::from_rotation_x(camera_angle(pos.z)) * vec3(0.0, 0.0, 1.0),
        vec3(0.0, 1.0, 0.0),
    )
}

pub fn camera_matrix(pos: Vec3, aspect: Float) -> Matrix4 {
    let projection = projection(aspect);
    let view = camera_view(pos);

    projection * view
}

pub fn lerp_coords_to_pixel(a: TileCoord, b: TileCoord, t: Float) -> Vec2 {
    let a = Vec2::new(a.x as Float, a.y as Float);
    let b = Vec2::new(b.x as Float, b.y as Float);
    let lerp = Vec2::lerp(a, b, t);

    let p = HEX_GRID_LAYOUT.fract_hex_to_world_pos(lerp);

    vec2(p.x, p.y)
}

/// Converts screen space coordinates into normalized coordinates.
#[inline]
pub fn screen_to_normalized((width, height): (Float, Float), c: Vec2) -> Vec2 {
    let size = vec2(width, height) * 0.5;

    let c = vec2(c.x, c.y);
    let c = c - size;
    let c = c / size;

    vec2(c.x, c.y)
}

/// Gets the hex position being pointed at.
#[inline]
pub fn main_pos_to_fract_hex(
    (width, height): (Float, Float),
    main_pos: Vec2,
    camera_pos: Vec3,
) -> Vec2 {
    let p = screen_to_world((width, height), main_pos, camera_pos);

    HEX_GRID_LAYOUT.world_pos_to_fract_hex(vec2(p.x as Float, p.y as Float))
}

/// Converts screen coordinates to world coordinates.
#[inline]
pub fn screen_to_world((width, height): (Float, Float), pos: Vec2, camera_pos: Vec3) -> Vec3 {
    let pos = screen_to_normalized((width, height), pos);

    normalized_to_world((width, height), pos, camera_pos)
}

/// Converts normalized screen coordinates to world coordinates.
#[inline]
pub fn normalized_to_world(
    (width, height): (Float, Float),
    normalized: Vec2,
    camera_pos: Vec3,
) -> Vec3 {
    let aspect = width / height;

    let matrix = camera_view(vec3(0.0, 0.0, camera_pos.z)).inverse() * projection(aspect).inverse();

    let pos = vec4(normalized.x, normalized.y, -1.0, 1.0);
    let pos = matrix * pos;
    let pos = pos.truncate() / pos.w;

    let end = vec4(normalized.x, normalized.y, 1.0, 1.0);
    let end = matrix * end;
    let end = end.truncate() / end.w;

    let ray = (end - pos).normalize();
    let normal = vec3(0.0, 0.0, -1.0);
    let d = -camera_pos.dot(normal) / ray.dot(normal);
    let p = ray * d;

    p + camera_pos
}

pub fn get_screen_world_bounding_vec(size: (Float, Float), camera_pos: Vec3) -> (Vec2, Vec2) {
    let a = normalized_to_world(size, vec2(-1.0, -1.0), camera_pos).truncate();
    let b = normalized_to_world(size, vec2(-1.0, 1.0), camera_pos).truncate();
    let c = normalized_to_world(size, vec2(1.0, -1.0), camera_pos).truncate();
    let d = normalized_to_world(size, vec2(1.0, 1.0), camera_pos).truncate();

    let min = a.min(b).min(c.min(d));
    let max = a.max(b).max(c.max(d));

    (min, max)
}

/// Gets the culling range from the camera's position
pub fn get_culling_range(size: (Float, Float), camera_pos: Vec3) -> TileBounds {
    let (bound_min, bound_max) = get_screen_world_bounding_vec(size, camera_pos);

    let size = bound_max - bound_min;
    let bound_center = size / 2.0 + bound_min;

    let size = HEX_GRID_LAYOUT.world_pos_to_hex((size / 2.0).ceil());

    TileBounds::new(
        HEX_GRID_LAYOUT.world_pos_to_hex(bound_center).into(),
        size.ulength(),
    )
}

#[inline]
pub fn direction_to_angle(d: Vec2) -> Float {
    let angle = d.y.atan2(d.x);

    angle.rem_euclid(std::f32::consts::PI)
}

pub fn tile_direction_to_angle(direction: TileCoord) -> Option<Float> {
    match direction {
        TileCoord::TOP_RIGHT => Some(0.0),
        TileCoord::RIGHT => Some(-60.0),
        TileCoord::BOTTOM_RIGHT => Some(-120.0),
        TileCoord::BOTTOM_LEFT => Some(-180.0),
        TileCoord::LEFT => Some(-240.0),
        TileCoord::TOP_LEFT => Some(-300.0),
        _ => None,
    }
}
