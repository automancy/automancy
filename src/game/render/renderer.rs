use actix::{Actor, Addr, Context, Handler, Message, MessageResponse, ResponseFuture};

use futures::{future::join, FutureExt};
use serde::{Deserialize, Serialize};

use crate::math::data::{Matrix4, Num, Point3};

use super::camera::{Camera, CameraPos, CameraPosRequest, View, ViewRequest};

#[derive(Message)]
#[rtype(result = "DrawInfo")]
pub struct Redraw {
    pub aspect: Num,
}

#[derive(Debug, Clone, MessageResponse)]
pub struct DrawInfo {
    pub pos: Point3,
    pub view: Matrix4,
}

impl DrawInfo {
    fn from(view: View, camera_pos: CameraPos) -> Self {
        Self {
            pos: camera_pos.0,
            view: view.0,
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
        let view = self
            .camera
            .send(ViewRequest { aspect: msg.aspect })
            .map(|v| v.unwrap());

        let camera_pos = self.camera.send(CameraPosRequest).map(|v| v.unwrap());

        join(view, camera_pos)
            .map(|(view, camera_pos)| DrawInfo::from(view, camera_pos))
            .boxed_local()
    }
}

impl Renderer {
    pub fn new(camera: Addr<Camera>) -> Self {
        Self { camera }
    }
}
