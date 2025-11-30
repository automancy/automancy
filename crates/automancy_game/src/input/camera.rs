use approx::abs_diff_eq;
use automancy_data::{
    game::coord::{TileCoord, TileCoordBounds},
    math::{Float, Matrix4, Rect, Vec2, Vec3, consts},
    rendering::view,
};

use crate::input::InputHandler;

#[inline]
#[must_use]
pub const fn default_fov() -> Float {
    consts::PI / 2.0
}

/// The game's camera system.
///
/// To get the correct behaviors, [`GameCamera::update`] should be run every frame.
///
/// You should never modify any of the fields directly, fields are marked `pub` for convenience.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameCamera {
    pub pos: Vec3,

    view_info: view::ViewInfo,

    cursor_pos: Vec2,
    move_vel: Vec2,
    scroll_vel: Float,

    pub transform: Matrix4,
    pub bounding_rect: Rect,
    pub view_center: Vec3,
    pub culling_bounds: TileCoordBounds,
    pub cursor_world_pos: Vec3,
    pub cursor_coord: TileCoord,
}

impl GameCamera {
    pub fn new(viewport_size: Vec2) -> Self {
        let mut this = Self {
            pos: Vec3::new(0.0, 0.0, 0.75),

            // dummy values, we update these values immediately down below.
            view_info: view::ViewInfo {
                fovy: consts::FRAC_PI_2,
                aspect: 1.0,
                viewport_size: Vec2::one(),
            },

            cursor_pos: Vec2::zero(),
            move_vel: Vec2::new(0.0, 0.0),
            scroll_vel: 0.0,

            transform: Matrix4::identity(),
            bounding_rect: Rect::new_empty(Vec2::zero()),
            view_center: Vec3::new(0.0, 0.0, 0.75),
            culling_bounds: TileCoordBounds::Empty,
            cursor_world_pos: Vec3::zero(),
            cursor_coord: TileCoord::ZERO,
        };

        this.set_viewport_size(viewport_size);
        this.set_fov(default_fov());

        this
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GameCamera {
    #[inline]
    #[track_caller]
    fn view_changed(&mut self) {
        self.cursor_world_pos = view::pixel_to_world(self.cursor_pos, self.pos, self.view_info);
        self.cursor_coord = TileCoord::from_world_pos(self.cursor_world_pos.xy());

        self.transform = view::camera_transform(self.pos, self.view_info);
        self.bounding_rect = view::viewport_bounding_rect_in_world(self.pos, self.view_info);
        self.view_center = view::normalized_to_world(Vec2::new(0.0, 0.0), self.pos, self.view_info);

        let culling_min = ((self.bounding_rect.min / 16.0).floor() * 16.0) - 8.0;
        let culling_max = ((self.bounding_rect.max / 16.0).ceil() * 16.0) + 8.0;

        self.culling_bounds = TileCoordBounds::offset_rect(
            TileCoord::offset_from_world_pos(culling_min),
            TileCoord::offset_from_world_pos(culling_max),
        );
    }

    /// Changes the camera's fov (field of view). Unit in radians.
    #[inline]
    #[track_caller]
    pub fn set_fov(&mut self, fovy: Float) {
        if self.view_info.fovy != fovy {
            self.view_info.fovy = fovy;
            self.view_changed();
        }
    }

    /// Changes the camera's viewport size. Unit in physical pixels.
    #[inline]
    #[track_caller]
    pub fn set_viewport_size(&mut self, viewport_size: Vec2) {
        if self.view_info.viewport_size != viewport_size {
            self.view_info.viewport_size = viewport_size;
            self.view_info.aspect = self.view_info.viewport_size.x / self.view_info.viewport_size.y;
            self.view_changed();
        }
    }

    /// Updates the camera's state. This function should be run every frame.
    #[inline]
    #[track_caller]
    pub fn update(&mut self, elapsed: Float) {
        let mut pos_changed = false;

        let (move_dir, move_mag) = self.move_vel.normalized_and_get_magnitude();
        if move_mag > 0.001 {
            self.pos += (move_dir * (move_mag * 120.0 * elapsed)).with_z(0.0) * view::camera_view_scale(self.view_info.viewport_size);

            self.move_vel -= move_dir * (move_mag * 5.0 * elapsed).min(1.0);
            pos_changed = true;
        } else {
            self.move_vel = Vec2::zero();
        }

        let (scroll_dir, scroll_mag) = (self.scroll_vel.signum(), self.scroll_vel.abs());
        if scroll_mag > 0.01 {
            self.pos.z += scroll_dir * (scroll_mag * 16.0 * elapsed);
            self.pos.z = self.pos.z.clamp(0.0, 1.0);

            self.scroll_vel -= scroll_dir * (scroll_mag * 8.0 * elapsed).min(1.0);
            pos_changed = true;
        } else {
            self.scroll_vel = 0.0;
        }

        if pos_changed {
            self.view_changed();
        }
    }

    /// Gets the camera's position in [`TileCoord`].
    #[inline]
    #[track_caller]
    pub fn get_tile_coord(&self) -> TileCoord {
        TileCoord::from_world_pos(self.pos.xy())
    }

    /// Updates the camera based on the input state.
    #[inline]
    #[track_caller]
    pub fn handle_input(&mut self, input: &InputHandler) {
        if self.cursor_pos != input.main_pos {
            self.cursor_pos = input.main_pos;

            self.view_changed();
        }

        if input.main_held
            && let Some(delta) = input.main_move
        {
            const MAX_MOVE_VEL: Float = 2.0;

            self.move_vel += -delta / 500.0;
            self.move_vel = self.move_vel.map(|v| v.clamp(-MAX_MOVE_VEL, MAX_MOVE_VEL));
        }

        if let Some(delta) = input.scroll {
            const MAX_SCROLL_VEL: Float = 0.2;

            let change = (-delta.x + -delta.y) * 1.3;
            if !abs_diff_eq!(change, 0.0) {
                self.scroll_vel += change;
                self.scroll_vel = self.scroll_vel.clamp(-MAX_SCROLL_VEL, MAX_SCROLL_VEL);
            }
        }
    }
}
