use std::sync::{Arc, Mutex};

use crate::math::data::Point3;
pub struct Player {
    pub pos: Arc<Mutex<Point3>>,
}
