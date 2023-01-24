use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::ops::{Div, Mul};

use cgmath::{point2, point3, vec3, Zero, vec2};
use cgmath::num_traits::clamp;

use crate::{
    game::{player::input::handler::InputState},
    math::{
        cg::{Matrix4, Num, Point3, Vector2, Vector3},
        util::perspective,
    },
};
use crate::game::render::data::FAR;
use crate::math::cg::Point2;



#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub pos: Point3,
    pub holding_main: bool,
    pub move_vel: Vector2,
    pub scroll_vel: Num,
    pub main_pos: Point2,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            holding_main: false,
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,
            main_pos: point2(0.0, 0.0)
        }
    }
}

impl CameraState {
    pub fn matrix(pos: Point3, aspect: Num) -> Matrix4 {
        let view = Self::view(pos);
        let projection = Self::projection(aspect);

        projection * view
    }

    pub fn eye(z: Num) -> Vector3 {
        let z = 1.0 - z;
        let r = z.mul(FRAC_PI_2).sin();
        let o = r.mul(PI / 2.25).cos();

        vec3(0.0, r, o)
    }

    pub fn actual_pos(pos: Point3, eye: Vector3) -> Point3 {
        point3(pos.x, pos.y, pos.z * 6.0 + eye.z)
    }

    pub fn view(pos: Point3) -> Matrix4 {
        let eye = Self::eye(pos.z);
        let actual_pos = Self::actual_pos(pos, eye);

        Matrix4::look_to_rh(actual_pos, eye, Vector3::unit_y())
    }

    pub fn projection(aspect: Num) -> Matrix4 {
        perspective(FRAC_PI_2, aspect, 0.01, 1000.0)
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

    fn scroll(z: Num, vel: Num) -> Num {
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
                pos.y += vel.y;
                pos.x += vel.x;

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
    fn on_scroll(&mut self, delta: Vector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = -y;

            self.camera_state.scroll_vel += change * 0.2;
        }
    }

    fn on_moving_main(&mut self, delta: Vector2) {
        self.camera_state.move_vel += delta / 250.0;
    }
}
