use cgmath::{point2, vec2};
use winit::event::{
    DeviceEvent, ElementState, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent,
};

use crate::math::cg::{Num, Point2, Vector2};

pub enum GameWindowEvent {
    MainPressed,
    MainReleased,
    MouseWheel { delta: Vector2 },
    ModifierChanged { modifier: ModifiersState },
    CursorPos { pos: Point2 },
}

pub enum GameDeviceEvent {
    MainMove { delta: Vector2 },
}

pub struct GameInputEvent {
    pub window: Option<GameWindowEvent>,
    pub device: Option<GameDeviceEvent>,
}

pub fn convert_input<'a>(
    window_event: Option<WindowEvent<'a>>,
    device_event: Option<DeviceEvent>,
) -> GameInputEvent {
    let mut window = None;
    let mut device = None;

    if let Some(event) = window_event {
        use GameWindowEvent::*;

        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                window = Some(match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(x, y);

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(delta.x as Num, delta.y as Num);

                        MouseWheel { delta }
                    }
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        window = Some(if state == ElementState::Pressed {
                            MainPressed
                        } else {
                            MainReleased
                        });
                    }
                    _ => (),
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                window = Some(ModifierChanged { modifier });
            }
            WindowEvent::CursorMoved { position, .. } => {
                window = Some(CursorPos {
                    pos: point2(position.x as f32, position.y as f32),
                });
            }
            _ => (),
        }
    }

    if let Some(event) = &device_event {
        use GameDeviceEvent::*;

        match event {
            DeviceEvent::MouseMotion { delta } => {
                let (x, y) = *delta;

                let delta = vec2(x as Num, y as Num);

                device = Some(MainMove { delta });
            }
            _ => (),
        }
    }

    GameInputEvent { window, device }
}
