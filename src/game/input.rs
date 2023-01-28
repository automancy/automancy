use cgmath::{point2, vec2};
use winit::event::{DeviceEvent, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event::ElementState::Pressed;

use crate::util::cg::{Double, DPoint2, DPoint3, DVector2};

#[derive(Debug, Copy, Clone)]
pub enum GameWindowEvent {
    None,
    MainPos { pos: DPoint2 },
    MainPressed,
    MainReleased,
    AlternatePressed,
    AlternateReleased,
    MouseWheel { delta: DVector2 },
    ModifierChanged { modifier: ModifiersState },
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
                    },
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
                window = ModifierChanged { modifier: *modifier };
            }
            WindowEvent::CursorMoved { position, .. } => {
                window = MainPos {
                    pos: point2(position.x, position.y),
                };
            }
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
            DeviceEvent::Key(keyboard_input)=> {
                match keyboard_input.virtual_keycode {
                    Some(VirtualKeyCode::Escape) => {
                        device = if keyboard_input.state == Pressed {
                            ExitPressed
                        } else {
                            ExitReleased
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    GameInputEvent { window, device }
}

#[derive(Debug, Copy, Clone)]
pub struct InputState {
    pub main_pos: DPoint2,

    pub main_held: bool,
    pub alternate_held: bool,
    pub exit_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,
    pub exit_pressed: bool,

    pub scroll: Option<DVector2>,

    pub main_move: Option<DVector2>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            main_pos: point2(0.0, 0.0),

            main_held: false,
            alternate_held: false,
            exit_held: false,

            main_pressed: false,
            alternate_pressed: false,
            exit_pressed: false,

            scroll: None,
            main_move: None,
        }
    }
}

impl InputState {
    pub fn reset(&mut self) {
        self.main_pressed = false;
        self.alternate_pressed = false;
        self.exit_pressed = false;

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
            GameWindowEvent::ModifierChanged { .. } => {}
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