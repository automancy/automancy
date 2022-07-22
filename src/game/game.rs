use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use winit::event::{DeviceEvent, WindowEvent};

use crate::registry::init::InitData;

use super::{
    data::{chunk::Chunk, pos::Pos},
    player::control::{MainClickListener, MainHoldListener, MainMoveListener},
};

pub struct Game {
    pub loaded_chunks: HashMap<Pos, Chunk>,
}

impl Game {
    pub fn window_event(
        &self,
        window_event: Option<WindowEvent>,
        device_event: Option<DeviceEvent>,
        init_data: &mut InitData,
        main_hold_listeners: &mut Vec<Arc<Mutex<dyn MainHoldListener>>>,
        main_click_listeners: &mut Vec<Arc<Mutex<dyn MainClickListener>>>,
        main_move_listeners: &mut Vec<Arc<Mutex<dyn MainMoveListener>>>,
    ) {
        init_data.tick(
            window_event,
            device_event,
            main_hold_listeners,
            main_click_listeners,
            main_move_listeners,
        );
    }
}
