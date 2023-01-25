use cgmath::{Zero, point3, point2, vec2};
use cgmath::num_traits::clamp;

use crate::{
    game::{player::input::handler::InputState},
};
use crate::math::cg::{Double, DPoint2, DPoint3, DVector2};

pub const FAR: Double = 0.0;

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub pos: DPoint3,
    pub holding_main: bool,
    pub move_vel: DVector2,
    pub scroll_vel: Double,
    pub main_pos: DPoint2,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            holding_main: false,
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,
            main_pos: point2(0.0, 0.0),
        }
    }
}

pub struct Camera {
    camera_state: CameraState,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            camera_state: Default::default(),
        }
    }
}

impl Camera {
    pub fn input_state(&mut self, input_state: InputState) {
        self.camera_state.holding_main = input_state.main_pressed;
        self.camera_state.main_pos = input_state.main_pos;

        if self.camera_state.holding_main {
            if let Some(delta) = input_state.main_move {
                self.on_moving_main(delta);
            }
        }

        if let Some(delta) = input_state.scroll {
            self.on_scroll(delta);
        }
    }

    fn scroll(z: Double, vel: Double) -> Double {
        let z = z + vel * 0.4;

        clamp(z, FAR, 1.0)
    }

    pub fn get_camera_state(&self) -> CameraState {
        self.camera_state
    }

    pub fn update_pos(&mut self) {
        let pos = &mut self.camera_state.pos;

        {
            let vel = &mut self.camera_state.move_vel;

            if !vel.is_zero() {
                pos.x += vel.x;
                pos.y += vel.y;

                *vel *= 0.9;
            }
        }

        {
            let vel = &mut self.camera_state.scroll_vel;
            if !vel.is_zero() {
                pos.z = Self::scroll(pos.z, *vel);

                *vel *= 0.7;
            }
        }
    }
}

impl Camera {
    fn on_scroll(&mut self, delta: DVector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = -y;

            self.camera_state.scroll_vel += change * 0.2;
        }
    }

    fn on_moving_main(&mut self, delta: DVector2) {
        self.camera_state.move_vel += delta / 250.0;
    }
}
