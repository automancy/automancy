use std::collections::HashSet;
use std::mem;

use serde::{Deserialize, Serialize};
use winit::event::ElementState::{Pressed, Released};
use winit::event::{
    DeviceEvent, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta,
    VirtualKeyCode, WindowEvent,
};

use automancy_defs::cgmath::{point2, vec2};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::math::{DPoint2, DVector2, Double};

use crate::options::Options;

pub static DEFAULT_KEYMAP: &[(VirtualKeyCode, KeyAction)] = &[
    (VirtualKeyCode::Z, actions::UNDO),
    (VirtualKeyCode::Escape, actions::ESCAPE),
    (VirtualKeyCode::F3, actions::DEBUG),
    (VirtualKeyCode::F11, actions::FULLSCREEN),
    (VirtualKeyCode::F1, actions::HIDE_GUI),
    (VirtualKeyCode::F2, actions::SCREENSHOT),
];

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum KeyActions {
    Escape,
    Undo,
    Debug,
    Fullscreen,
    Screenshot,
    HideGui,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PressTypes {
    Tap,    // returns true when the key is pressed once and will not press again until released
    Hold,   // returns true whenever the key is down
    Toggle, // pressing the key will either toggle it on or off
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct KeyAction {
    pub action: KeyActions,
    pub press_type: PressTypes,
}

pub mod actions {
    use super::{KeyAction, KeyActions, PressTypes};

    pub static ESCAPE: KeyAction = KeyAction {
        action: KeyActions::Escape,
        press_type: PressTypes::Tap,
    };
    pub static UNDO: KeyAction = KeyAction {
        action: KeyActions::Undo,
        press_type: PressTypes::Tap,
    };
    pub static DEBUG: KeyAction = KeyAction {
        action: KeyActions::Debug,
        press_type: PressTypes::Toggle,
    };
    pub static FULLSCREEN: KeyAction = KeyAction {
        action: KeyActions::Fullscreen,
        press_type: PressTypes::Toggle,
    };
    pub static SCREENSHOT: KeyAction = KeyAction {
        action: KeyActions::Screenshot,
        press_type: PressTypes::Tap,
    };
    pub static HIDE_GUI: KeyAction = KeyAction {
        action: KeyActions::HideGui,
        press_type: PressTypes::Toggle,
    };
}

/// The various controls of the game.
#[derive(Debug, Copy, Clone)]
pub enum GameInputEvent {
    None,
    MainPos { pos: DPoint2 },
    MainMove { delta: DVector2 },
    MouseWheel { delta: DVector2 },
    MainPressed,
    MainReleased,
    AlternatePressed,
    AlternateReleased,
    TertiaryPressed,
    TertiaryReleased,
    ExitPressed,
    ExitReleased,
    ModifierChanged { modifier: ModifiersState },
    KeyboardEvent { input: KeyboardInput },
}

pub fn convert_input(
    window_event: Option<&WindowEvent>,
    device_event: Option<&DeviceEvent>,
    (width, height): (Double, Double),
    sensitivity: Double,
) -> GameInputEvent {
    let mut result = GameInputEvent::None;

    if let Some(event) = window_event {
        use GameInputEvent::*;

        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                result = match delta {
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(
                            delta.x / width * sensitivity,
                            delta.y / height * sensitivity,
                        );

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(*x as Double * sensitivity, *y as Double * sensitivity);

                        MouseWheel { delta }
                    }
                };
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        result = if state == &Pressed {
                            MainPressed
                        } else {
                            MainReleased
                        };
                    }
                    MouseButton::Right => {
                        result = if state == &Pressed {
                            AlternatePressed
                        } else {
                            AlternateReleased
                        };
                    }
                    MouseButton::Middle => {
                        result = if state == &Pressed {
                            TertiaryPressed
                        } else {
                            TertiaryReleased
                        }
                    }
                    _ => {}
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                result = ModifierChanged {
                    modifier: *modifier,
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = point2(position.x, position.y);

                result = MainPos { pos };
            }
            WindowEvent::KeyboardInput { input, .. } => result = KeyboardEvent { input: *input },
            _ => {}
        }
    }

    if let Some(event) = device_event {
        use GameInputEvent::*;

        if let DeviceEvent::MouseMotion { delta } = event {
            let delta = vec2(delta.0 * sensitivity, -delta.1 * sensitivity);

            result = MainMove { delta };
        }
    }

    result
}

#[derive(Debug, Clone)]
pub struct InputHandler {
    pub main_pos: DPoint2,
    pub scroll: Option<DVector2>,
    pub main_move: Option<DVector2>,

    pub main_held: bool,
    pub alternate_held: bool,
    pub tertiary_held: bool,

    pub control_held: bool,
    pub shift_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,
    pub tertiary_pressed: bool,

    pub key_map: HashMap<VirtualKeyCode, KeyAction>,
    pub key_states: HashSet<KeyActions>,

    to_clear: Vec<KeyAction>,
}

impl InputHandler {
    pub fn new(options: &Options) -> Self {
        Self {
            main_pos: point2(0.0, 0.0),
            scroll: None,
            main_move: None,

            main_held: false,
            alternate_held: false,
            tertiary_held: false,

            control_held: false,
            shift_held: false,

            main_pressed: false,
            alternate_pressed: false,
            tertiary_pressed: false,

            key_map: options.keymap.clone(),
            key_states: Default::default(),

            to_clear: Default::default(),
        }
    }

    pub fn reset(&mut self) {
        self.main_pressed = false;
        self.alternate_pressed = false;
        self.tertiary_pressed = false;

        self.main_move = None;
        self.scroll = None;

        for v in mem::take(&mut self.to_clear) {
            self.key_states.remove(&v.action);
        }
    }

    pub fn update(&mut self, event: GameInputEvent) {
        match event {
            GameInputEvent::MainPos { pos } => {
                self.main_pos = pos;
            }
            GameInputEvent::MainMove { delta } => {
                self.main_move = Some(delta);
            }
            GameInputEvent::MouseWheel { delta } => {
                self.scroll = Some(delta);
            }
            GameInputEvent::MainPressed => {
                self.main_pressed = true;
                self.main_held = true;
            }
            GameInputEvent::MainReleased => {
                self.main_held = false;
            }
            GameInputEvent::AlternatePressed => {
                self.alternate_pressed = true;
                self.alternate_held = true;
            }
            GameInputEvent::AlternateReleased => {
                self.alternate_held = false;
            }
            GameInputEvent::TertiaryPressed => {
                self.tertiary_pressed = true;
                self.tertiary_held = true;
            }
            GameInputEvent::TertiaryReleased => {
                self.tertiary_held = false;
            }
            GameInputEvent::ModifierChanged { modifier } => {
                self.shift_held = false;
                self.control_held = false;

                if modifier.contains(ModifiersState::SHIFT) {
                    self.shift_held = true;
                }
                if modifier.contains(ModifiersState::CTRL) {
                    self.control_held = true;
                }
            }
            GameInputEvent::KeyboardEvent {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(virtual_keycode),
                        ..
                    },
            } => {
                self.handle_key(state, virtual_keycode);
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, state: ElementState, key: VirtualKeyCode) -> Option<()> {
        let action = *self.key_map.get(&key)?;

        match action.press_type {
            PressTypes::Tap => match state {
                Pressed => {
                    self.key_states.insert(action.action);
                    self.to_clear.push(action);
                }
                Released => {
                    self.key_states.remove(&action.action);
                }
            },
            PressTypes::Hold => match state {
                Pressed => {
                    self.key_states.insert(action.action);
                }
                Released => {
                    self.key_states.remove(&action.action);
                }
            },
            PressTypes::Toggle => match state {
                Pressed => {
                    if self.key_states.contains(&action.action) {
                        self.key_states.remove(&action.action);
                    } else {
                        self.key_states.insert(action.action);
                    }
                }
                Released => {}
            },
        }

        Some(())
    }

    pub fn key_active(&self, action: &KeyActions) -> bool {
        self.key_states.contains(action)
    }
}
