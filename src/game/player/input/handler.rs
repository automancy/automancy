use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    watch,
};
use winit::event::ModifiersState;

use crate::{
    game::player::input::primitive::{GameDeviceEvent, GameWindowEvent},
    math::cg::{Num, Point2, Vector2},
};

use super::primitive::GameInputEvent;

#[derive(Debug, Clone, Copy)]
pub struct InputState {
    pub main_clicked: bool,
    pub main_pos: Point2,
    pub main_hold: Option<Num>,
    pub main_move: Option<Vector2>,
    pub scroll: Option<Vector2>,
    pub modifier_shift: bool,
}

pub struct InputHandler {
    main_clicked: bool,
    main_pos: Point2,
    main_last_clicked: u32,

    modifier: ModifiersState, // TODO maybe make a custom type for this?

    send_input_state: watch::Sender<Option<InputState>>,
}

impl InputHandler {
    pub fn new() -> (Self, watch::Receiver<Option<InputState>>) {
        let (send_input_state, recv_input_state) = watch::channel(None);

        let it = Self {
            main_clicked: false,
            main_pos: Point2::new(0.0, 0.0),
            main_last_clicked: 0,

            modifier: ModifiersState::empty(),

            send_input_state,
        };

        (it, recv_input_state)
    }
}

impl InputHandler {
    pub fn send(&mut self, event: GameInputEvent) {
        let mut scroll = None;

        if let Some(event) = event.window {
            use GameWindowEvent::*;

            match event {
                MainPressed => {
                    self.main_clicked = true;
                }
                MainReleased => {
                    self.main_clicked = false;
                }
                MouseWheel { delta } => {
                    scroll = Some(delta);
                }
                ModifierChanged { modifier } => {
                    self.modifier = modifier;
                }
                CursorPos { pos } => {
                    self.main_pos = pos;
                }
            }
        }

        let mut modifier_shift = false;

        if self.modifier.shift() {
            modifier_shift = true;
        }

        let mut main_move = None;

        if let Some(event) = event.device {
            use GameDeviceEvent::*;

            match event {
                MainMove { delta } => {
                    main_move = Some(delta);
                }
            }
        }

        let mut main_hold = None;

        if self.main_last_clicked > 0 {
            let elapsed = (self.main_last_clicked as Num) / 60.0; // TODO get FPS

            main_hold = Some(elapsed);
        }

        if self.main_clicked {
            self.main_clicked = true;

            self.main_last_clicked += 1;
        } else {
            self.main_last_clicked = 0;
        }

        self.send_input_state.send_replace(Some(InputState {
            main_clicked: self.main_clicked,
            main_pos: self.main_pos,
            main_hold,
            main_move,
            scroll,
            modifier_shift,
        }));
    }
}
