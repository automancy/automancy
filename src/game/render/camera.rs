use std::{
    f32::consts,
    ops::{Div, Mul},
};

use actix::{Actor, Context, Handler, Message, MessageResponse};
use cgmath::{point3, vec3, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    game::{game::GameState, player::input_handler::InputState},
    math::{
        data::{rad, Matrix4, Num, Point3, Rad, Vector2, Vector3},
        util::perspective,
    },
};

const MAX_CAMERA_Z: f32 = 4.0;
const EPSILON_ZERO: f32 = f32::EPSILON;

#[derive(Message)]
#[rtype(result = "CameraState")]
pub struct CameraRequest {
    pub aspect: Num,
}

#[derive(Debug, Clone, Serialize, Deserialize, MessageResponse)]
pub struct CameraState {
    pub pos: Point3,
    pub view: Matrix4,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pos: Point3::new(0.0, 0.0, 0.0),
            view: Matrix4::zero(),
        }
    }
}

pub struct Camera {
    pos: Point3,

    rotation: Rad,
    holding_main: bool,
    move_vel: Vector2,
    scroll_vel: Num,
}

impl Actor for Camera {
    type Context = Context<Self>;
}

impl Handler<CameraRequest> for Camera {
    type Result = CameraState;

    fn handle(&mut self, msg: CameraRequest, _ctx: &mut Self::Context) -> Self::Result {
        CameraState {
            pos: self.pos,
            view: self.view(msg.aspect),
        }
    }
}

impl Handler<InputState> for Camera {
    type Result = Option<()>;

    fn handle(&mut self, state: InputState, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(_) = state.main_hold {
            self.on_holding_main();
        } else {
            self.on_not_holding_main();
        }

        if let Some(delta) = state.main_move {
            self.on_moving_main(delta);
        }

        if let Some(delta) = state.scroll {
            self.on_scroll(delta);
        }

        Some(())
    }
}

fn scroll(z: Num, vel: Num) -> Num {
    let z = z + vel;

    if z < EPSILON_ZERO {
        return 0.0;
    }

    if z >= MAX_CAMERA_Z {
        return MAX_CAMERA_Z;
    }

    return z;
}

impl Handler<GameState> for Camera {
    type Result = Option<()>;

    fn handle(&mut self, _msg: GameState, _ctx: &mut Self::Context) -> Self::Result {
        let pos = &mut self.pos;

        {
            let vel = &mut self.move_vel;
            if !vel.is_zero() {
                pos.x += vel.x;
                pos.y += vel.y;

                *vel -= *vel * 0.05;
            }
        }

        {
            let vel = &mut self.scroll_vel;
            if !vel.is_zero() {
                pos.z = scroll(pos.z, *vel);

                *vel -= *vel * 0.2;
            }
        }

        Some(())
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),

            holding_main: false,
            rotation: rad(0.0),
            move_vel: Vector2::zero(),
            scroll_vel: 0.0,
        }
    }

    fn view(&self, aspect: Num) -> Matrix4 {
        let pos = self.pos;
        let z = pos.z.div(MAX_CAMERA_Z);
        let r = -z.mul(consts::FRAC_PI_2).sin();
        let o = r.mul(consts::PI / 2.25).cos();

        let actual_pos = point3(pos.x / 100.0, -pos.y / 100.0, MAX_CAMERA_Z - pos.z + o);
        // TODO ??????? / 100.0

        let projection = perspective(consts::FRAC_PI_2, aspect, 0.1, 100.0);
        let view = Matrix4::look_to_rh(actual_pos, vec3(0.0, r, o), -Vector3::unit_y());

        projection * view
    }

    fn on_scroll(&mut self, delta: Vector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = y;

            self.scroll_vel += change / 7.5;
        }
    }

    fn on_holding_main(&mut self) {
        if !self.holding_main {
            self.holding_main = true;
        }
    }

    fn on_not_holding_main(&mut self) {
        if self.holding_main {
            self.holding_main = false;
        }
    }

    fn on_moving_main(&mut self, delta: Vector2) {
        if self.holding_main {
            self.move_vel += delta / 25.0;
        }
    }
}
