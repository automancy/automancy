use std::collections::HashMap;

use winit::event::{DeviceEvent, WindowEvent};

use super::{
    data::{chunk::Chunk, pos::Pos},
    player::player::Player,
};

pub struct Game {
    pub loaded_chunks: HashMap<Pos, Chunk>,

    pub player: Player,
}

impl Game {
    pub fn window_event(
        &mut self,
        window_event: Option<WindowEvent>,
        device_event: Option<DeviceEvent>,
    ) {
        self.player.handle_events(window_event, device_event);
    }
}
