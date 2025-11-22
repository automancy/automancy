pub mod colors;
pub mod draw;
pub mod view;

pub mod util {
    use crate::math::{Float, Matrix4, Vec2, Vec3, vec2_to_degrees};

    pub const LINE_DEPTH: Float = 0.075;

    /// Produces a line shape.
    #[inline]
    pub fn make_line(a: Vec2, b: Vec2, z: Float) -> Matrix4 {
        let mid = Vec2::lerp(a, b, 0.5);
        let d = a.distance(b);
        let theta = vec2_to_degrees(b - a);

        Matrix4::translation_3d(Vec3::new(mid.x, mid.y, z))
            * Matrix4::rotation_z(theta)
            * Matrix4::scaling_3d(Vec3::new(d.max(0.001), 0.1, LINE_DEPTH))
    }
}
