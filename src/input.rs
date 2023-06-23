use std::collections::HashSet;
use std::mem;

use serde::{Deserialize, Serialize};

use automancy_defs::cg::{DPoint2, DVector2, Double};
use automancy_defs::cgmath::{point2, vec2};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::winit::event::ElementState::{Pressed, Released};
use automancy_defs::winit::event::{
    DeviceEvent, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta,
    VirtualKeyCode, WindowEvent,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum KeyActions {
    ESCAPE,
    UNDO,
    DEBUG,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PressTypes {
    ONESHOT, // returns true when the key is pressed once and will not press again until released
    HOLD,    // returns true whenever the key is down
    TOGGLE,  // pressing the key will either toggle it on or off
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct KeyAction {
    pub action: KeyActions,
    pub press_type: PressTypes,
}

pub mod actions {
    use super::{KeyAction, KeyActions, PressTypes};

    pub static ESCAPE: KeyAction = KeyAction {
        action: KeyActions::ESCAPE,
        press_type: PressTypes::ONESHOT,
    };
    pub static UNDO: KeyAction = KeyAction {
        action: KeyActions::UNDO,
        press_type: PressTypes::ONESHOT,
    };
    pub static DEBUG: KeyAction = KeyAction {
        action: KeyActions::DEBUG,
        press_type: PressTypes::TOGGLE,
    };
}

/// The various controls of the game.
#[derive(Debug, Copy, Clone)]
pub enum GameWindowEvent {
    /// no keys pressed
    None,
    /// mouse cursor moved
    MainPos { pos: DPoint2 },
    /// mouse 1 pressed
    MainPressed,
    /// mouse 1 released
    MainReleased,
    /// mouse 2 pressed
    AlternatePressed,
    /// mouse 2 released
    AlternateReleased,
    /// mouse wheel scrolled
    MouseWheel { delta: DVector2 },
    /// modifier key pressed
    ModifierChanged { modifier: ModifiersState },
    /// keyboard event
    KeyboardEvent { input: KeyboardInput },
}

#[derive(Debug, Copy, Clone)]
pub enum GameDeviceEvent {
    None,
    MainMove { delta: DVector2 },
    ExitPressed,
    ExitReleased,
}

#[derive(Debug, Copy, Clone)]
pub struct GameInputEvent {
    pub window: GameWindowEvent,
    pub device: GameDeviceEvent,
}

pub fn convert_input(
    window_event: Option<&WindowEvent>,
    device_event: Option<&DeviceEvent>,
) -> GameInputEvent {
    let mut window = GameWindowEvent::None;
    let mut device = GameDeviceEvent::None;

    if let Some(event) = window_event {
        use GameWindowEvent::*;

        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                window = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(*x as Double, *y as Double);

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(delta.x, delta.y);

                        MouseWheel { delta }
                    }
                };
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        window = if state == &Pressed {
                            MainPressed
                        } else {
                            MainReleased
                        };
                    }
                    MouseButton::Right => {
                        window = if state == &Pressed {
                            AlternatePressed
                        } else {
                            AlternateReleased
                        };
                    }
                    _ => {}
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                window = ModifierChanged {
                    modifier: *modifier,
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                window = MainPos {
                    pos: point2(position.x, position.y),
                };
            }
            WindowEvent::KeyboardInput { input, .. } => window = KeyboardEvent { input: *input },
            _ => {}
        }
    }

    if let Some(event) = device_event {
        use GameDeviceEvent::*;

        if let DeviceEvent::MouseMotion { delta } = event {
            let (x, y) = delta;

            let delta = vec2(*x, -*y);

            device = MainMove { delta };
        }
    }

    GameInputEvent { window, device }
}

#[derive(Debug, Clone)]
pub struct InputHandler {
    pub main_pos: DPoint2,
    pub scroll: Option<DVector2>,
    pub main_move: Option<DVector2>,

    pub main_held: bool,
    pub control_held: bool,
    pub alternate_held: bool,
    pub shift_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,

    pub key_map: HashMap<VirtualKeyCode, KeyAction>,
    pub key_states: HashSet<KeyAction>,

    to_clear: Vec<KeyAction>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self {
            main_pos: point2(0.0, 0.0),
            scroll: None,
            main_move: None,

            main_held: false,
            control_held: false,
            alternate_held: false,
            shift_held: false,

            main_pressed: false,
            alternate_pressed: false,

            key_map: HashMap::from([
                (VirtualKeyCode::Z, actions::UNDO),
                (VirtualKeyCode::Escape, actions::ESCAPE),
                (VirtualKeyCode::F3, actions::DEBUG),
            ]),
            key_states: Default::default(),

            to_clear: Default::default(),
        }
    }
}

impl InputHandler {
    pub fn reset(&mut self) {
        self.main_pressed = false;
        self.alternate_pressed = false;
        self.main_move = None;
        self.scroll = None;

        mem::take(&mut self.to_clear).into_iter().for_each(|v| {
            self.key_states.remove(&v);
        });
    }

    pub fn update(&mut self, event: GameInputEvent) {
        match event.window {
            GameWindowEvent::MainPos { pos } => {
                self.main_pos = pos;
            }
            GameWindowEvent::MainPressed => {
                self.main_pressed = true;
                self.main_held = true;
            }
            GameWindowEvent::MainReleased => {
                self.main_held = false;
            }
            GameWindowEvent::AlternatePressed => {
                self.alternate_pressed = true;
                self.alternate_held = true;
            }
            GameWindowEvent::AlternateReleased => {
                self.alternate_held = false;
            }
            GameWindowEvent::MouseWheel { delta } => {
                self.scroll = Some(delta);
            }
            GameWindowEvent::ModifierChanged { modifier } => {
                self.shift_held = false;
                self.control_held = false;

                if modifier.contains(ModifiersState::SHIFT) {
                    self.shift_held = true;
                }
                if modifier.contains(ModifiersState::CTRL) {
                    self.control_held = true;
                }
            }
            GameWindowEvent::KeyboardEvent {
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
        if let GameDeviceEvent::MainMove { delta } = event.device {
            self.main_move = Some(delta);
        }
    }

    pub fn handle_key(&mut self, state: ElementState, key: VirtualKeyCode) -> Option<()> {
        let action = *self.key_map.get(&key)?;

        match action.press_type {
            PressTypes::ONESHOT => match state {
                Pressed => {
                    self.key_states.insert(action);
                    self.to_clear.push(action);
                }
                Released => {
                    self.key_states.remove(&action);
                }
            },
            PressTypes::HOLD => match state {
                Pressed => {
                    self.key_states.insert(action);
                }
                Released => {
                    self.key_states.remove(&action);
                }
            },
            PressTypes::TOGGLE => match state {
                Pressed => {
                    if self.key_states.contains(&action) {
                        self.key_states.remove(&action);
                    } else {
                        self.key_states.insert(action);
                    }
                }
                Released => {}
            },
        }

        Some(())
    }

    pub fn key_pressed(&self, action: &KeyAction) -> bool {
        self.key_states.contains(action)
    }
}
