use approx::abs_diff_eq;

use crate::math::{Float, Matrix3, Matrix4, Rect, Vec2, Vec3, Vec4, consts};

pub const WORLD_PLANE_Z: Float = 0.0;
pub const WORLD_FORWARD_DIR: Vec3 = Vec3::new(0.0, 0.0, -1.0);
pub const WORLD_NORMAL: Vec3 = Vec3::new(-WORLD_FORWARD_DIR.x, -WORLD_FORWARD_DIR.y, -WORLD_FORWARD_DIR.z);

#[inline]
#[must_use]
pub const fn z_near() -> Float {
    0.01
}

#[inline]
#[must_use]
pub const fn fov() -> Float {
    consts::PI / 2.0
}

#[inline]
#[must_use]
pub fn perspective_rh_oz(fovy: Float, aspect: Float, near: Float) -> Matrix4 {
    let f = 1.0 / Float::tan(0.5 * fovy);
    Matrix4::from_col_arrays([
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, 0.0, -1.0],
        [0.0, 0.0, near, 0.0],
    ])
}

#[inline]
#[must_use]
pub fn projection(aspect: Float) -> Matrix4 {
    perspective_rh_oz(fov(), aspect, z_near())
}

#[inline]
#[must_use]
const fn transform_camera_z(z: Float) -> Float {
    let mut z = z * 4.0;

    if z > 1.0 && z <= 1.5 {
        z = 1.0
    } else if z > 1.5 {
        z -= 0.5
    }

    z * 4.0 + 2.8
}

#[inline]
#[must_use]
const fn transform_camera_pos(Vec3 { x, y, z }: Vec3) -> Vec3 {
    Vec3::new(x, y, transform_camera_z(z))
}

#[inline]
#[must_use]
pub const fn camera_angle(z: Float) -> Float {
    // TODO magic values
    let max = 6.5;

    if z < max {
        let normalized = (max - z) / 4.0;

        normalized / -1.5
    } else {
        0.0
    }
}

#[inline]
#[must_use]
fn view_forward_dir(camera_pos: Vec3) -> Vec3 {
    Matrix3::rotation_x(-camera_angle(camera_pos.z)) * WORLD_FORWARD_DIR
}

#[inline]
#[must_use]
fn camera_view(camera_pos: Vec3) -> Matrix4 {
    let camera_pos = transform_camera_pos(camera_pos);

    Matrix4::look_at_rh(camera_pos, camera_pos + view_forward_dir(camera_pos), Vec3::new(0.0, 1.0, 0.0))
}

#[inline]
#[must_use]
fn camera_view_inverted(camera_pos: Vec3) -> Matrix4 {
    let camera_pos = transform_camera_pos(camera_pos);

    Matrix4::model_look_at_rh(camera_pos, camera_pos + view_forward_dir(camera_pos), Vec3::new(0.0, 1.0, 0.0))
}

#[inline]
#[must_use]
pub fn camera_matrix(aspect: Float, camera_pos: Vec3) -> Matrix4 {
    let projection = projection(aspect);
    let view = camera_view(camera_pos);

    projection * view
}

/// Converts screen space coordinates into normalized coordinates.
#[inline]
#[must_use]
pub fn pixel_to_normalized(pos: Vec2, viewport_size: Vec2) -> Vec2 {
    let size = Vec2::new(viewport_size.x, viewport_size.y) * 0.5;

    let pos = Vec2::new(pos.x, pos.y);
    let pos = pos - size;
    let pos = pos / size;

    Vec2::new(pos.x, pos.y)
}

/// Converts screen coordinates to world coordinates.
#[inline]
#[must_use]
pub fn pixel_to_world(pos: Vec2, viewport_size: Vec2, camera_pos: Vec3) -> Vec3 {
    let pos = pixel_to_normalized(pos, viewport_size);

    normalized_to_world(pos, viewport_size.x / viewport_size.y, camera_pos)
}

/// Converts normalized screen coordinates to world coordinates.
#[inline]
#[must_use]
pub fn normalized_to_world(pos: Vec2, aspect: Float, camera_pos: Vec3) -> Vec3 {
    // Pretend camera_pos=(0, 0) for better accuracy.
    let matrix = camera_view_inverted(Vec3::new(0.0, 0.0, camera_pos.z)) * projection(aspect).inverted();

    let start = Vec4::new(pos.x, -pos.y, -1.0, 1.0);
    let start = matrix * start;
    let start = start.xyz() / start.w;

    let end = Vec4::new(pos.x, -pos.y, 1.0, 1.0);
    let end = matrix * end;
    let end = end.xyz() / end.w;

    // Line intersection
    let l = (end - start).normalized();
    let l_0 = start;
    let p_0 = Vec3::new(0.0, 0.0, WORLD_PLANE_Z);

    // The formula is `d = ((p_0 - l_0) ⋅ n) / (l ⋅ n)`, where p_0 is a point on the plane, l_0 is a point on the line,
    // l is the line, and n is the normal vector of the plane
    let f = l.dot(WORLD_NORMAL);
    if abs_diff_eq!(f, 0.0) {
        panic!("View direction must not be parallel to the world plane.");
    }

    let d = (p_0 - l_0).dot(WORLD_NORMAL) / f;

    // `p = l_0 + l * d`
    let p = l_0 + l * d;

    // We're pretending the line was shot from some point in view space with camera_pos=(0, 0), so we need to add back the camera position.
    // We also add back the z value, because the ray was "shot" from behind the camera at z=-1, and the camera is at z=0
    p + camera_pos
}

#[inline]
#[must_use]
pub fn viewport_bounding_rect_in_world(viewport_size: Vec2, camera_pos: Vec3) -> Rect {
    let aspect = viewport_size.x / viewport_size.y;
    let a = normalized_to_world(Vec2::new(-1.0, -1.0), aspect, camera_pos).xy();
    let b = normalized_to_world(Vec2::new(-1.0, 1.0), aspect, camera_pos).xy();
    let c = normalized_to_world(Vec2::new(1.0, -1.0), aspect, camera_pos).xy();
    let d = normalized_to_world(Vec2::new(1.0, 1.0), aspect, camera_pos).xy();

    let p = [a, b, c, d];
    let min_x = p.into_iter().map(|v| v.x).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
    let min_y = p.into_iter().map(|v| v.y).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
    let max_x = p.into_iter().map(|v| v.x).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
    let max_y = p.into_iter().map(|v| v.y).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();

    let min = Vec2::new(min_x, min_y);
    let max = Vec2::new(max_x, max_y);

    Rect { min, max }
}
