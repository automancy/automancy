use cgmath::{point2, vec2};
use winit::event::ModifiersState;

use crate::{
    game::player::input::primitive::{GameDeviceEvent, GameWindowEvent},
};
use crate::math::cg::{DPoint2, DVector2};

use super::primitive::GameInputEvent;

#[derive(Debug, Clone, Copy)]
pub struct InputState {
    pub main_pos: DPoint2,
    pub main_pressed: bool,
    pub main_move: Option<DVector2>,

    pub scroll: Option<DVector2>,
    pub modifier_shift: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            main_pos: point2(0.0, 0.0),
            main_pressed: false,
            main_move: None,

            scroll: None,
            modifier_shift: false,
        }
    }
}

pub struct InputHandler {
    main_pos: DPoint2,
    main_pressed: bool,
    main_last_clicked: u32,

    modifier: ModifiersState, // TODO maybe make a custom type for this?
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            main_pos: point2(0.0, 0.0),
            main_pressed: false,
            main_last_clicked: 0,

            modifier: ModifiersState::empty(),
        }
    }
}

impl InputHandler {
    pub fn update(&mut self, event: GameInputEvent) -> InputState {
        let mut scroll = None;

        if let Some(event) = event.window {
            use GameWindowEvent::*;

            match event {
                MainPressed => {
                    self.main_pressed = true;
                }
                MainReleased => {
                    self.main_pressed = false;
                }
                ModifierChanged { modifier } => {
                    self.modifier = modifier;
                }
                CursorPos { pos } => {
                    self.main_pos = pos;
                }
                MouseWheel { delta } => {
                    scroll = Some(delta);
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
                    main_move = Some(vec2(delta.x, -delta.y));
                }
            }
        }

        if self.main_pressed {
            self.main_pressed = true;

            self.main_last_clicked += 1;
        } else {
            self.main_last_clicked = 0;
        }

        InputState {
            main_pressed: self.main_pressed,
            main_pos: self.main_pos,
            main_move,
            scroll,
            modifier_shift,
        }
    }
}