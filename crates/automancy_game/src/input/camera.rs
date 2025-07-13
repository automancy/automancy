use std::ops::Mul;

use approx::abs_diff_eq;
use automancy_data::{
    game::coord::{OffsetCoord, TileBounds, TileCoord},
    math::{Float, Matrix4, Vec2, Vec3},
    rendering,
};

use crate::input::InputHandler;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameCamera {
    pos: Vec3,
    move_vel: Vec2,
    scroll_vel: Float,

    matrix: Matrix4,

    pub culling_bounds: TileBounds,
    pub pointing_at: TileCoord,
}

impl GameCamera {
    fn update_state(&mut self, viewport_size: Vec2) {
        self.matrix = rendering::view::camera_matrix(viewport_size.x / viewport_size.y, self.pos);

        let bounding_rect = rendering::view::viewport_bounding_rect_in_world(viewport_size, self.pos);
        self.culling_bounds = TileBounds::offset_rect(
            TileCoord::offset_from_world_pos(bounding_rect.min.round()) - OffsetCoord::new(2, 2),
            TileCoord::offset_from_world_pos(bounding_rect.max.round()) + OffsetCoord::new(2, 2),
        );
    }

    pub fn new(viewport_size: Vec2) -> Self {
        let mut this = Self {
            pos: Vec3::new(0.0, 0.0, 0.75),
            move_vel: Vec2::new(0.0, 0.0),
            scroll_vel: 0.0,

            matrix: Matrix4::identity(),

            culling_bounds: TileBounds::Empty,
            pointing_at: TileCoord::new(0, 0),
        };

        this.update_state(viewport_size);

        this
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> Vec3 {
        self.pos
    }

    pub fn get_matrix(&self) -> Matrix4 {
        self.matrix
    }
}

impl GameCamera {
    /// Sets the position the camera is centered on.
    pub fn update_pointing_at(&mut self, viewport_size: Vec2, main_pos: Vec2) {
        let world_pos = rendering::view::pixel_to_world(main_pos, viewport_size, self.pos);

        self.pointing_at = TileCoord::from_world_pos(world_pos.xy());
    }

    /// Gets the TileCoord the camera is pointing at.
    pub fn get_tile_coord(&self) -> TileCoord {
        TileCoord::from_world_pos(Vec2::new(self.pos.x as Float, self.pos.y as Float))
    }

    /// Updates the movement state of the camera based on input.
    pub fn handle_input(&mut self, input: &InputHandler) {
        if input.main_held
            && let Some(delta) = input.main_move
        {
            self.on_moving_main(delta);
        }

        if let Some(delta) = input.scroll {
            self.on_scroll(delta);
        }
    }

    /// Updates the camera's position.
    pub fn update_pos(&mut self, viewport_size: Vec2, elapsed: Float) {
        if self.move_vel.magnitude_squared() > 0.0000001 {
            let m = elapsed * 100.0;

            self.pos.x += self.move_vel.x * m;
            self.pos.y += self.move_vel.y * m;

            self.move_vel -= self.move_vel * elapsed.mul(4.0).min(0.9);
        }

        if self.scroll_vel.abs() > 0.00005 {
            let m = elapsed * 20.0;

            self.pos.z += self.scroll_vel * m;
            self.pos.z = self.pos.z.clamp(0.0, 1.0);

            self.scroll_vel -= self.scroll_vel * elapsed.mul(15.0).min(0.9);
        }

        self.update_state(viewport_size);
    }

    /// Called when the camera is scrolled.
    fn on_scroll(&mut self, delta: Vec2) {
        const MAX_SCROLL_VEL: Float = 0.2;

        let change = (-delta.x + -delta.y) * 1.3;
        if !abs_diff_eq!(change, 0.0) {
            self.scroll_vel += change;
            self.scroll_vel = self.scroll_vel.clamp(-MAX_SCROLL_VEL, MAX_SCROLL_VEL);
        }
    }

    /// Called when the camera is moving.
    fn on_moving_main(&mut self, delta: Vec2) {
        const MAX_MOVE_VEL: Float = 2.0;

        self.move_vel += -delta / 500.0;
        self.move_vel = self.move_vel.map(|v| v.clamp(-MAX_MOVE_VEL, MAX_MOVE_VEL));
    }
}
