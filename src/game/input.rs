use cgmath::{point2, vec2};
use winit::event::ElementState::Pressed;
use winit::event::{
    DeviceEvent, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};

use crate::util::cg::{DPoint2, DVector2, Double};

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
            DeviceEvent::Key(keyboard_input) => {
                if let Some(VirtualKeyCode::Escape) = keyboard_input.virtual_keycode {
                    device = if keyboard_input.state == Pressed {
                        ExitPressed
                    } else {
                        ExitReleased
                    }
                }
            }
            _ => {}
        }
    }

    GameInputEvent { window, device }
}

#[derive(Debug, Copy, Clone)]
pub struct InputHandler {
    pub main_pos: DPoint2,

    pub main_held: bool,
    pub control_held: bool,
    pub alternate_held: bool,
    pub exit_held: bool,
    pub shift_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,
    pub exit_pressed: bool,

    pub undo_pressed: bool,

    pub scroll: Option<DVector2>,

    pub main_move: Option<DVector2>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self {
            main_pos: point2(0.0, 0.0),

            main_held: false,
            control_held: false,
            alternate_held: false,
            exit_held: false,
            shift_held: false,

            main_pressed: false,
            alternate_pressed: false,
            exit_pressed: false,

            undo_pressed: false,

            scroll: None,
            main_move: None,
        }
    }
}

impl InputHandler {
    pub fn reset(&mut self) {
        self.main_pressed = false;
        self.alternate_pressed = false;
        self.exit_pressed = false;

        self.undo_pressed = false;

        self.main_move = None;
        self.scroll = None;
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
                if input.state == Pressed {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Z) => self.undo_pressed = true,
                        _ => {}
                    }
                }
            }
            GameWindowEvent::None => {}
        }

        match event.device {
            GameDeviceEvent::MainMove { delta } => {
                self.main_move = Some(delta);
            }
            GameDeviceEvent::ExitPressed => {
                self.exit_pressed = true;
                self.exit_held = true;
            }
            GameDeviceEvent::ExitReleased => {
                self.exit_held = false;
            }
            GameDeviceEvent::None => {}
        }
    }
}
