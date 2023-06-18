use automancy_defs::cg::{DPoint2, DVector2, Double};
use automancy_defs::cgmath::{point2, vec2};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::winit::event::ElementState::Pressed;
use automancy_defs::winit::event::{
    DeviceEvent, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum KeyActions {
    UNDO,
    DEBUG,
    PAUSE,
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
            _ => (),
        }
    }

    if let Some(event) = device_event {
        use GameDeviceEvent::*;

        match event {
            DeviceEvent::MouseMotion { delta } => {
                let (x, y) = delta;

                let delta = vec2(*x, -*y);

                device = MainMove { delta };
            }
            _ => {}
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

    pub keymap: HashMap<u32, KeyActions>,
    pub keystates: HashMap<KeyActions, bool>,
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

            keymap: HashMap::from([
                (VirtualKeyCode::Z as u32, KeyActions::UNDO),
                (VirtualKeyCode::Escape as u32, KeyActions::PAUSE),
                (VirtualKeyCode::F3 as u32, KeyActions::DEBUG),
            ]),
            keystates: HashMap::from([
                (KeyActions::UNDO, false),
                (KeyActions::DEBUG, false),
                (KeyActions::PAUSE, false),
            ]),
        }
    }
}
lazy_static! {
    static ref DEFAULT_KEYSTATE: HashMap<KeyActions, bool> = HashMap::from([
        (KeyActions::UNDO, false),
        (KeyActions::DEBUG, false),
        (KeyActions::PAUSE, false),
    ]);
}
impl InputHandler {
    pub fn reset(&mut self) {
        self.main_pressed = false;
        self.alternate_pressed = false;
        self.main_move = None;
        self.scroll = None;
        self.keystates = DEFAULT_KEYSTATE.clone(); //i feel like a clone here is okay
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
            GameWindowEvent::KeyboardEvent { input } => {
                if input.state == Pressed && input.virtual_keycode.is_some() {
                    let action = self.keymap.get(&(input.virtual_keycode.unwrap() as u32));
                    if action.is_some() {
                        self.set_action(action.unwrap());
                    }
                }
            }
            GameWindowEvent::None => {}
        }

        match event.device {
            GameDeviceEvent::MainMove { delta } => {
                self.main_move = Some(delta);
            }
            _ => {}
        }
    }
    pub fn set_action(&mut self, action: &KeyActions) {
        *self.keystates.get_mut(action).unwrap() = true;
    }
    pub fn key_pressed(&self, action: &KeyActions) -> bool {
        self.keystates.get(action).unwrap().clone()
    }
}
