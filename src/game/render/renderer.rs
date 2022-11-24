use cgmath::point2;
use futures::{future::join, FutureExt};
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    watch,
};

use crate::{
    game::player::input::handler::InputState,
    math::cg::{Num, Point2},
};

#[derive(Debug, Clone, Copy)]
pub struct RendererState {
    pub aspect: Num,

    pub window_size: Point2,
    pub cursor_pos: Point2,
}

pub struct Renderer {
    aspect: Num,
    window_size: Point2,

    main_pos: Point2,

    send_renderer_state: Sender<RendererState>,
    recv_input_state: watch::Receiver<Option<InputState>>,
}

impl Renderer {
    pub fn new(
        recv_input_state: watch::Receiver<Option<InputState>>,
    ) -> (Self, Receiver<RendererState>) {
        let (send_renderer_state, recv_renderer_state) = channel(2);

        let it = Self {
            aspect: 1.0,
            window_size: point2(1.0, 1.0),

            main_pos: point2(0.0, 0.0),

            send_renderer_state,
            recv_input_state,
        };

        (it, recv_renderer_state)
    }
}

impl Renderer {
    pub fn send(&self) {
        self.send_renderer_state
            .send(RendererState {
                aspect: self.aspect,
                window_size: self.window_size,
                cursor_pos: self.main_pos,
            })
            .unwrap();
    }

    pub fn recv(&mut self) {
        if let Some(state) = *self.recv_input_state.borrow_and_update() {
            self.main_pos = state.main_pos;
        }
    }

    pub fn update(&mut self, aspect: Num, window_size: Point2) {
        self.aspect = aspect;
        self.window_size = window_size;
    }
}
