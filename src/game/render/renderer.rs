use actix::{Actor, Addr, Context, Handler, Message, MessageResponse, ResponseFuture};

use futures::FutureExt;
use serde::{Deserialize, Serialize};

use crate::math::data::{Matrix4, Num, Point3};

use super::camera::{Camera, CameraRequest, CameraState};

#[derive(Message)]
#[rtype(result = "DrawInfo")]
pub struct Redraw {
    pub aspect: Num,
}

#[derive(Debug, Serialize, Deserialize, Clone, MessageResponse)]
pub struct DrawInfo {
    pub pos: Point3,
    pub view: Matrix4,
}

impl DrawInfo {
    fn from_camera(state: CameraState) -> Self {
        Self {
            pos: state.pos,
            view: state.view,
        }
    }
}

pub struct Renderer {
    camera: Addr<Camera>,
}

impl Actor for Renderer {
    type Context = Context<Self>;
}

impl Handler<Redraw> for Renderer {
    type Result = ResponseFuture<DrawInfo>;

    fn handle(&mut self, msg: Redraw, _ctx: &mut Self::Context) -> Self::Result {
        self.camera
            .send(CameraRequest { aspect: msg.aspect })
            .map(|v| DrawInfo::from_camera(v.unwrap()))
            .boxed_local()
    }
}

impl Renderer {
    pub fn new(camera: Addr<Camera>) -> Self {
        Self { camera }
    }
}
