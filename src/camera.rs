use std::ops::Mul;

use egui::NumExt;

use automancy_defs::coord::TileCoord;
use automancy_defs::glam::{dvec2, dvec3, vec2};
use automancy_defs::hexx::{Hex, HexBounds};
use automancy_defs::math;
use automancy_defs::math::{matrix, DMatrix4, DVec2, DVec3, Double, Float, HEX_GRID_LAYOUT};

use crate::input::InputHandler;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pos: DVec3,
    move_vel: DVec2,
    scroll_vel: Double,

    pub culling_range: HexBounds,
    pub pointing_at: TileCoord,
    matrix: DMatrix4,
}

pub fn fit_z(mut z: Double) -> Double {
    if z > 1.0 {
        if z <= 1.5 {
            z = 1.0
        } else {
            z -= 0.5
        }
    }

    2.5 + z * 4.0
}

pub fn fit_pos(DVec3 { x, y, z }: DVec3) -> DVec3 {
    dvec3(x, y, fit_z(z))
}

impl Camera {
    pub fn new((width, height): (Double, Double)) -> Self {
        let pos = dvec3(0.0, 0.0, 2.0);
        let matrix = matrix(fit_pos(pos), width / height);

        Self {
            pos,
            move_vel: dvec2(0.0, 0.0),
            scroll_vel: 0.0,

            culling_range: math::get_culling_range((width, height), fit_pos(pos)),
            pointing_at: TileCoord::new(0, 0),
            matrix,
        }
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> DVec3 {
        fit_pos(self.pos)
    }

    pub fn get_matrix(&self) -> DMatrix4 {
        self.matrix
    }
}

impl Camera {
    /// Sets the position the camera is centered on.
    pub fn update_pointing_at(&mut self, main_pos: DVec2, (width, height): (Double, Double)) {
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

    /// Updates the movement state of the camera based on control input.
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
    pub fn update_pos(&mut self, (width, height): (Double, Double), elapsed: Double) {
        let m = elapsed * 100.0;

        if self.move_vel.length_squared() > 0.0000001 {
            self.pos.x += self.move_vel.x * m;
            self.pos.y += self.move_vel.y * m;

            self.move_vel -= self.move_vel * elapsed.mul(4.0).at_most(0.9);
        }

        if self.scroll_vel.abs() > 0.00005 {
            self.pos.z += self.scroll_vel * m;
            //self.pos.z = self.pos.z.clamp(0.05, 4.0);
            self.pos.z = self.pos.z.clamp(1.0, 4.0);

            self.scroll_vel -= self.scroll_vel * elapsed.mul(15.0).at_most(0.9);
        }

        self.matrix = matrix(self.get_pos(), width / height);
        self.culling_range = math::get_culling_range((width, height), self.get_pos());
    }

    /// Called when the camera is scrolled.
    fn on_scroll(&mut self, delta: DVec2) {
        const MAX_SCROLL_VEL: Double = 0.2;

        let y = delta.y;

        if y != 0.0 {
            let change = -y;

            self.scroll_vel += change / 20.0;
            self.scroll_vel = self.scroll_vel.clamp(-MAX_SCROLL_VEL, MAX_SCROLL_VEL);
        }
    }

    /// Called when the camera is moving.
    fn on_moving_main(&mut self, delta: DVec2) {
        const MAX_MOVE_VEL: Double = 2.0;

        self.move_vel += delta / 600.0;
        self.move_vel = self.move_vel.clamp(
            dvec2(-MAX_MOVE_VEL, -MAX_MOVE_VEL),
            dvec2(MAX_MOVE_VEL, MAX_MOVE_VEL),
        );
    }
}
