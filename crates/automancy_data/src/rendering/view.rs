use crate::math::{Float, Matrix3, Matrix4, Rect, Vec2, Vec3, Vec4};

pub const WORLD_PLANE_Z: Float = 0.0;
pub const WORLD_FORWARD_DIR: Vec3 = Vec3::new(0.0, 0.0, -1.0);
pub const WORLD_NORMAL: Vec3 = Vec3::new(-WORLD_FORWARD_DIR.x, -WORLD_FORWARD_DIR.y, -WORLD_FORWARD_DIR.z);

pub const Z_NEAR: Float = 0.01;
pub const Z_FAR: Float = 40.0;
const NATIVE_HEIGHT: Float = 800.0;

/// Contains information about the camera view.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewInfo {
    /// the field of view
    pub fovy: Float,
    /// the aspect ratio of the viewport size
    pub aspect: Float,
    /// the viewport size
    pub viewport_size: Vec2,
}

#[inline]
#[must_use]
#[rustfmt::skip]
fn perspective_rh(fovy: Float, a: Float, near: Float, far: Float) -> Matrix4 {
    let (sin_fov, cos_fov) = (0.5 * fovy).sin_cos();
    let h = cos_fov / sin_fov;
    let w = h / a;
    let r = far / (near - far);

    Matrix4::from_col_arrays([
        [w, 0.0, 0.0, 0.0],
        [0.0, h, 0.0, 0.0],
        [0.0, 0.0, r, -1.0],
        [0.0, 0.0, r * near, 0.0],
    ])
}

#[inline]
#[must_use]
pub fn camera_projection(fovy: Float, aspect: Float) -> Matrix4 {
    perspective_rh(fovy, aspect, Z_FAR, Z_NEAR)
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

    z * 4.0 + 2.0
}

#[inline]
#[must_use]
const fn transform_camera_pos(
    Vec3 {
        x,
        y,
        z,
    }: Vec3,
) -> Vec3 {
    Vec3::new(x, y, transform_camera_z(z))
}

/// The direction the camera will be looking at.
#[inline]
#[must_use]
fn view_forward_dir(camera_pos: Vec3) -> Vec3 {
    // TODO magic values
    const THRESHOLD: Float = 6.0;
    let x_angle = if camera_pos.z < THRESHOLD {
        (THRESHOLD - camera_pos.z) / 8.0
    } else {
        0.0
    };

    Matrix3::rotation_x(x_angle) * WORLD_FORWARD_DIR
}

#[inline]
#[must_use]
fn camera_view(camera_pos: Vec3) -> Matrix4 {
    Matrix4::look_at_rh(camera_pos, camera_pos + view_forward_dir(camera_pos), Vec3::new(0.0, 1.0, 0.0))
}

#[inline]
#[must_use]
fn camera_view_inverted(camera_pos: Vec3) -> Matrix4 {
    Matrix4::model_look_at_rh(camera_pos, camera_pos + view_forward_dir(camera_pos), Vec3::new(0.0, 1.0, 0.0))
}

/// Scales the tiles according to some [native height](NATIVE_HEIGHT) value so they stay consistent regardless of viewport size.
#[inline]
#[must_use]
pub const fn camera_view_scale(viewport_size: Vec2) -> Vec3 {
    let scale = NATIVE_HEIGHT / viewport_size.y;

    Vec3::new(scale, scale, 1.0)
}

/// Inverse of [`camera_view_scale`].
#[inline]
#[must_use]
pub const fn camera_view_scale_inverted(viewport_size: Vec2) -> Vec3 {
    let scale = viewport_size.y * const { 1.0 / NATIVE_HEIGHT };

    Vec3::new(scale, scale, 1.0)
}

#[inline]
#[must_use]
pub fn camera_transform(
    camera_pos: Vec3,
    ViewInfo {
        fovy,
        aspect,
        viewport_size,
    }: ViewInfo,
) -> Matrix4 {
    let camera_pos = transform_camera_pos(camera_pos);

    let projection = camera_projection(fovy, aspect);
    let view = camera_view(camera_pos);

    projection * Matrix4::scaling_3d(camera_view_scale(viewport_size)) * view
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
pub fn pixel_to_world(
    pos: Vec2,
    camera_pos: Vec3,
    view_info @ ViewInfo {
        viewport_size, ..
    }: ViewInfo,
) -> Vec3 {
    let pos = pixel_to_normalized(pos, viewport_size);

    normalized_to_world(pos, camera_pos, view_info)
}

/// Converts normalized screen coordinates to world coordinates.
#[inline]
#[must_use]
pub fn normalized_to_world(
    pos: Vec2,
    camera_pos: Vec3,
    ViewInfo {
        fovy,
        aspect,
        viewport_size,
    }: ViewInfo,
) -> Vec3 {
    let camera_pos = transform_camera_pos(camera_pos);

    // Pretend camera_pos=(0, 0) for better accuracy.
    let matrix = camera_view_inverted(Vec3::new(0.0, 0.0, camera_pos.z))
        * Matrix4::scaling_3d(camera_view_scale_inverted(viewport_size))
        * camera_projection(fovy, aspect).inverted();

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
    let f = l.dot(WORLD_NORMAL).min(-0.05);

    let d = (p_0 - l_0).dot(WORLD_NORMAL) / f;

    // `p = l_0 + l * d`
    let p = l_0 + l * d;

    // We're pretending the line was shot from some point in view space with camera_pos=(0, 0), so we need to add back the camera position.
    // We also add back the z value, because the ray was "shot" from behind the camera at z=-1, and the camera is at z=0
    p + camera_pos
}

#[inline]
#[must_use]
pub fn viewport_bounding_rect_in_world(camera_pos: Vec3, view_info: ViewInfo) -> Rect {
    let a = normalized_to_world(Vec2::new(-1.0, -1.0), camera_pos, view_info).xy();
    let b = normalized_to_world(Vec2::new(-1.0, 1.0), camera_pos, view_info).xy();
    let c = normalized_to_world(Vec2::new(1.0, -1.0), camera_pos, view_info).xy();
    let d = normalized_to_world(Vec2::new(1.0, 1.0), camera_pos, view_info).xy();

    let p = [a, b, c, d];
    let min_x = p
        .into_iter()
        .map(|v| v.x)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let min_y = p
        .into_iter()
        .map(|v| v.y)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_x = p
        .into_iter()
        .map(|v| v.x)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_y = p
        .into_iter()
        .map(|v| v.y)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();

    let min = Vec2::new(min_x, min_y);
    let max = Vec2::new(max_x, max_y);

    Rect {
        min,
        max,
    }
}
