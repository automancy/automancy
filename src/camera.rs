use std::ops::Mul;

use automancy_defs::glam::{vec2, vec3, Vec2, Vec3};
use automancy_defs::hexx::Hex;
use automancy_defs::math;
use automancy_defs::math::{camera_matrix, Float, HEX_GRID_LAYOUT};
use automancy_defs::{
    coord::{TileBounds, TileCoord},
    math::Matrix4,
};

use crate::input::InputHandler;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pos: Vec3,
    move_vel: Vec2,
    scroll_vel: Float,

    pub culling_range: TileBounds,
    pub pointing_at: TileCoord,
    matrix: Matrix4,
}

pub fn fit_z(mut z: Float) -> Float {
    if z > 1.0 {
        if z <= 1.5 {
            z = 1.0
        } else {
            z -= 0.5
        }
    }

    2.5 + z * 4.0
}

pub fn fit_pos(Vec3 { x, y, z }: Vec3) -> Vec3 {
    vec3(x, y, fit_z(z))
}

impl Camera {
    pub fn new((width, height): (Float, Float)) -> Self {
        let pos = vec3(0.0, 0.0, 2.0);
        let matrix = camera_matrix(fit_pos(pos), width / height);

        Self {
            pos,
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,

            culling_range: math::get_culling_range((width, height), fit_pos(pos)),
            pointing_at: TileCoord::new(0, 0),
            matrix,
        }
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> Vec3 {
        fit_pos(self.pos)
    }

    pub fn get_matrix(&self) -> Matrix4 {
        self.matrix
    }
}

impl Camera {
    /// Sets the position the camera is centered on.
    pub fn update_pointing_at(&mut self, main_pos: Vec2, (width, height): (Float, Float)) {
        let p = Hex::round(
            math::main_pos_to_fract_hex((width, height), main_pos, self.get_pos()).to_array(),
        );

        self.pointing_at = p.into();
    }

    /// Gets the TileCoord the camera is pointing at.
    pub fn get_tile_coord(&self) -> TileCoord {
        HEX_GRID_LAYOUT
            .world_pos_to_hex(vec2(self.pos.x as Float, self.pos.y as Float))
            .into()
    }

    /// Updates the movement state of the camera based on input.
    pub fn handle_input(&mut self, input: &InputHandler) {
        if input.tertiary_held {
            if let Some(delta) = input.main_move {
                self.on_moving_main(delta);
            }
        }

        if let Some(delta) = input.scroll {
            self.on_scroll(delta);
        }
    }

    /// Updates the camera's position.
    pub fn update_pos(&mut self, (width, height): (Float, Float), elapsed: Float) {
        let m = elapsed * 100.0;

        if self.move_vel.length_squared() > 0.0000001 {
            self.pos.x += self.move_vel.x * m;
            self.pos.y += self.move_vel.y * m;

            self.move_vel -= self.move_vel * elapsed.mul(4.0).min(0.9);
        }

        if self.scroll_vel.abs() > 0.00005 {
            self.pos.z += self.scroll_vel * m;
            self.pos.z = self.pos.z.clamp(0.05, 4.0);

            self.scroll_vel -= self.scroll_vel * elapsed.mul(15.0).min(0.9);
        }

        self.matrix = camera_matrix(self.get_pos(), width / height);
        self.culling_range = math::get_culling_range((width, height), self.get_pos());
    }

    /// Called when the camera is scrolled.
    fn on_scroll(&mut self, delta: Vec2) {
        const MAX_SCROLL_VEL: Float = 0.2;

        let y = delta.y;

        if y != 0.0 {
            let change = -y;

            self.scroll_vel += change / 20.0;
            self.scroll_vel = self.scroll_vel.clamp(-MAX_SCROLL_VEL, MAX_SCROLL_VEL);
        }
    }

    /// Called when the camera is moving.
    fn on_moving_main(&mut self, delta: Vec2) {
        const MAX_MOVE_VEL: Float = 2.0;

        self.move_vel += delta / 500.0;
        self.move_vel = self.move_vel.clamp(
            vec2(-MAX_MOVE_VEL, -MAX_MOVE_VEL),
            vec2(MAX_MOVE_VEL, MAX_MOVE_VEL),
        );
    }
}
