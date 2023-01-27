use cgmath::{point2, vec2};
use winit::event::{DeviceEvent, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event::ElementState::Pressed;

use crate::math::cg::{Double, DPoint2, DVector2};

#[derive(Debug, Copy, Clone)]
pub enum GameWindowEvent {
    MainPressed,
    MainReleased,
    MouseWheel { delta: DVector2 },
    ModifierChanged { modifier: ModifiersState },
    CursorPos { pos: DPoint2 },
}

#[derive(Debug, Copy, Clone)]
pub enum GameDeviceEvent {
    MainMove { delta: DVector2 },
    ExitPressed,
}

#[derive(Debug, Copy, Clone)]
pub struct GameInputEvent {
    pub window: Option<GameWindowEvent>,
    pub device: Option<GameDeviceEvent>,
}

pub fn convert_input(
    window_event: Option<&WindowEvent>,
    device_event: Option<&DeviceEvent>,
) -> GameInputEvent {
    let mut window = None;
    let mut device = None;

    if let Some(event) = window_event {
        use GameWindowEvent::*;

        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                window = Some(match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(*x as Double, *y as Double);

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(delta.x, delta.y);

                        MouseWheel { delta }
                    }
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        window = Some(if state == &Pressed {
                            MainPressed
                        } else {
                            MainReleased
                        });
                    },
                    _ => {}
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                window = Some(ModifierChanged { modifier: *modifier });
            }
            WindowEvent::CursorMoved { position, .. } => {
                window = Some(CursorPos {
                    pos: point2(position.x, position.y),
                });
            }
            _ => (),
        }
    }

    if let Some(event) = device_event {
        use GameDeviceEvent::*;

        match event {
            DeviceEvent::MouseMotion { delta } => {
                let (x, y) = delta;

                let delta = vec2(*x, *y);

                device = Some(MainMove { delta });
            }
            DeviceEvent::Key(keyboard_input)=> {
                if keyboard_input.state == Pressed {
                    match keyboard_input.virtual_keycode {
                        Some(VirtualKeyCode::Escape) => {
                            device = Some(ExitPressed);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    GameInputEvent { window, device }
}
