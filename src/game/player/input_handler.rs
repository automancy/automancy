use actix::{Actor, Context, Handler, Message, Recipient};
use cgmath::vec2;

use serde::{Deserialize, Serialize};
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use crate::math::data::{Num, Vector2};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameWindowEvent {
    MainPressed,
    MainReleased,
    MouseWheel { delta: Vector2 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameDeviceEvent {
    MainMove { delta: Vector2 },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "Option<()>")]
pub struct GameInputEvent(Option<GameWindowEvent>, Option<GameDeviceEvent>);

pub fn convert_input(
    window_event: Option<WindowEvent<'_>>,
    device_event: Option<DeviceEvent>,
) -> GameInputEvent {
    let mut window = None;
    let mut device = None;

    if let Some(event) = window_event {
        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                window = Some(match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(x, y);

                        GameWindowEvent::MouseWheel { delta }
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(delta.x as f32, delta.y as f32);

                        GameWindowEvent::MouseWheel { delta }
                    }
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        window = Some(if state == ElementState::Pressed {
                            GameWindowEvent::MainPressed
                        } else {
                            GameWindowEvent::MainReleased
                        });
                    }
                    _ => (),
                };
            }
            _ => (),
        }
    }

    if let Some(event) = &device_event {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let delta = vec2(delta.0 as f32, delta.1 as f32);

                device = Some(GameDeviceEvent::MainMove { delta });
            }
            _ => (),
        }
    }

    GameInputEvent(window, device)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "Option<()>")]
pub struct InputState {
    pub main_click: bool,
    pub main_hold: Option<Num>,
    pub main_move: Option<Vector2>,
    pub scroll: Option<Vector2>,
}

pub struct InputHandler {
    main_clicked: bool,
    main_last_clicked: u32,

    camera: Recipient<InputState>,
}

impl Actor for InputHandler {
    type Context = Context<Self>;
}

impl InputHandler {
    pub fn new(camera: Recipient<InputState>) -> Self {
        Self {
            main_clicked: false,
            main_last_clicked: 0,

            camera,
        }
    }
}

impl Handler<GameInputEvent> for InputHandler {
    type Result = Option<()>;

    fn handle(&mut self, input: GameInputEvent, _ctx: &mut Self::Context) -> Self::Result {
        let mut state = InputState::default();

        let (window_event, device_event) = (input.0.as_ref(), input.1.as_ref());

        if let Some(event) = window_event {
            match event {
                GameWindowEvent::MainPressed => {
                    self.main_clicked = true;
                }
                GameWindowEvent::MainReleased => {
                    self.main_clicked = false;
                }
                GameWindowEvent::MouseWheel { delta } => {
                    state.scroll = Some(*delta);
                }
            }
        }

        if let Some(event) = device_event {
            match event {
                GameDeviceEvent::MainMove { delta } => {
                    state.main_move = Some(*delta);
                }
            }
        }

        if self.main_last_clicked > 0 {
            let elapsed = (self.main_last_clicked as Num) / 60.0; // TODO get FPS

            state.main_hold = Some(elapsed);
        }

        if self.main_clicked {
            self.main_clicked = true;

            self.main_last_clicked += 1;
        } else {
            self.main_last_clicked = 0;
        }

        self.camera.do_send(state);

        Some(())
    }
}
