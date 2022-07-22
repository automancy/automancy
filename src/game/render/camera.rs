use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
};

use cgmath::{perspective, point3, vec3, Angle, InnerSpace, Rad};

use crate::{
    game::{
        data::id::{Id, Identifiable},
        player::control::{MainHoldListener, MainMoveListener},
    },
    math::data::{Matrix4, Point3, Vector3},
};

pub struct Camera {
    pub pos: Arc<Mutex<Point3>>,
    pub rotation: Rad<f32>,

    pub holding_main: bool,
}

impl Camera {
    pub fn view(&self) -> Matrix4 {
        let pos = *self.pos.lock().unwrap();

        let pos = point3(pos.x, -pos.y, pos.z) / 100.0;

        println!("{:?}", pos);

        Matrix4::look_to_lh(pos, vec3(0.0, 0.0, -1.0), -Vector3::unit_y())
    }

    pub fn projection(&self) -> Matrix4 {
        perspective(Rad(1.0), 1.0, 0.001, 1.0)
    }
}

impl Identifiable for Camera {
    fn id(&self) -> Id {
        Id::automancy("Camera".to_string())
    }
}

impl_cmp!(Camera);

impl PartialOrd<Id> for Camera {
    fn partial_cmp(&self, _other: &Id) -> Option<Ordering> {
        Some(Ordering::Less)
    }
}

impl MainHoldListener for Camera {
    fn on_holding_main(&mut self, _elapsed: f32) {
        if !self.holding_main {
            log::debug!("held main on camera");
            self.holding_main = true;
        }
    }

    fn on_not_holding_main(&mut self) {
        self.holding_main = false;
    }
}

impl MainMoveListener for Camera {
    fn on_moving_main(&mut self, delta: (f64, f64)) {
        if self.holding_main {
            let mut pos = self.pos.lock().unwrap();
            pos.x += delta.0 as f32;
            pos.y += delta.1 as f32;

            log::debug!("{:?}", pos);
        }
    }
}
