use std::mem;

use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use winit::event::ElementState::{Pressed, Released};
use winit::event::{
    DeviceEvent, ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::keyboard::{Key, NamedKey, SmolStr};

use automancy_defs::glam::dvec2;
use automancy_defs::math::{DVec2, Double};

use crate::options::Options;

pub static DEFAULT_KEYMAP: &[(Key, KeyAction)] = &[
    (Key::Character(SmolStr::new_inline("z")), actions::UNDO),
    (Key::Character(SmolStr::new_inline("e")), actions::PLAYER),
    (Key::Named(NamedKey::Escape), actions::ESCAPE),
    (Key::Named(NamedKey::F1), actions::HIDE_GUI),
    (Key::Named(NamedKey::F2), actions::SCREENSHOT),
    (Key::Named(NamedKey::F3), actions::DEBUG),
    (Key::Named(NamedKey::F11), actions::FULLSCREEN),
];

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum KeyActions {
    Escape,
    Undo,
    Debug,
    Fullscreen,
    Screenshot,
    HideGui,
    Player,
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
        press_type: PressTypes::Tap,
    };
    pub static SCREENSHOT: KeyAction = KeyAction {
        action: KeyActions::Screenshot,
        press_type: PressTypes::Tap,
    };
    pub static HIDE_GUI: KeyAction = KeyAction {
        action: KeyActions::HideGui,
        press_type: PressTypes::Toggle,
    };
    pub static PLAYER: KeyAction = KeyAction {
        action: KeyActions::Player,
        press_type: PressTypes::Toggle,
    };
}

/// The various controls of the game.
#[derive(Debug, Clone)]
pub enum GameInputEvent {
    None,
    MainPos { pos: DVec2 },
    MainMove { delta: DVec2 },
    MouseWheel { delta: DVec2 },
    MainPressed,
    MainReleased,
    AlternatePressed,
    AlternateReleased,
    TertiaryPressed,
    TertiaryReleased,
    ExitPressed,
    ExitReleased,
    ModifierChanged { modifier: Modifiers },
    KeyboardEvent { event: KeyEvent },
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
                        let delta = dvec2(
                            delta.x / width * sensitivity,
                            delta.y / height * sensitivity,
                        );

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = dvec2(*x as Double * sensitivity, *y as Double * sensitivity);

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
                let pos = dvec2(position.x, position.y);

                result = MainPos { pos };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                result = KeyboardEvent {
                    event: event.clone(),
                }
            }
            _ => {}
        }
    }

    if let Some(event) = device_event {
        use GameInputEvent::*;

        if let DeviceEvent::MouseMotion { delta } = event {
            let delta = dvec2(delta.0 * sensitivity, -delta.1 * sensitivity);

            result = MainMove { delta };
        }
    }

    result
}

#[derive(Debug, Clone)]
pub struct InputHandler {
    pub main_pos: DVec2,
    pub scroll: Option<DVec2>,
    pub main_move: Option<DVec2>,

    pub main_held: bool,
    pub alternate_held: bool,
    pub tertiary_held: bool,

    pub control_held: bool,
    pub shift_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,
    pub tertiary_pressed: bool,

    pub key_map: HashMap<Key, KeyAction>,
    pub key_states: HashSet<KeyActions>,

    to_clear: Vec<KeyAction>,
}

impl InputHandler {
    pub fn new(options: &Options) -> Self {
        Self {
            main_pos: dvec2(0.0, 0.0),
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

                if modifier.state().shift_key() {
                    self.shift_held = true;
                }
                if modifier.state().control_key() {
                    self.control_held = true;
                }
            }
            GameInputEvent::KeyboardEvent {
                event: KeyEvent {
                    state, logical_key, ..
                },
            } => {
                self.handle_key(state, logical_key);
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, state: ElementState, key: Key) -> Option<()> {
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

    pub fn key_active(&self, action: KeyActions) -> bool {
        self.key_states.contains(&action)
    }
}
