use actix::{Actor, Context, Handler, Message, MessageResponse, Recipient};
use cgmath::{point2, vec2};

use winit::event::{
    DeviceEvent, ElementState, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent,
};

use crate::math::data::{Num, Point2, Vector2};

#[derive(Debug, Default, Clone, Message)]
#[rtype(result = "Option<()>")]
pub struct InputState {
    pub main_click: bool,
    pub main_hold: Option<Num>,
    pub main_move: Option<Vector2>,
    pub scroll: Option<Vector2>,
    pub modifier_shift: bool,
}

pub struct InputHandler {
    main_clicked: bool,
    main_last_clicked: u32,

    modifier: ModifiersState,

    camera: Recipient<InputState>,

    cursor_pos: Point2,
}

impl Actor for InputHandler {
    type Context = Context<Self>;
}

impl InputHandler {
    pub fn new(camera: Recipient<InputState>) -> Self {
        Self {
            main_clicked: false,
            main_last_clicked: 0,

            modifier: ModifiersState::empty(),

            camera,

            cursor_pos: Point2::new(0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameWindowEvent {
    MainPressed,
    MainReleased,
    MouseWheel { delta: Vector2 },
    ModifierChanged { modifier: ModifiersState },
    CursorPos { pos: Point2 },
}

#[derive(Debug, Clone)]
pub enum GameDeviceEvent {
    MainMove { delta: Vector2 },
}

#[derive(Debug, Default, Clone, Message)]
#[rtype(result = "CursorState")]
pub struct CursorStateRequest;

#[derive(MessageResponse)]
pub struct CursorState {
    pub pos: Point2,
}

impl Handler<CursorStateRequest> for InputHandler {
    type Result = CursorState;

    fn handle(&mut self, _msg: CursorStateRequest, _ctx: &mut Self::Context) -> Self::Result {
        CursorState {
            pos: self.cursor_pos,
        }
    }
}

#[derive(Debug, Default, Clone, Message)]
#[rtype(result = "Option<()>")]
pub struct GameInputEvent(Option<GameWindowEvent>, Option<GameDeviceEvent>);

pub fn convert_input<'a>(
    window_event: Option<&'a WindowEvent<'a>>,
    device_event: Option<&'a DeviceEvent>,
) -> GameInputEvent {
    let mut window = None;
    let mut device = None;

    if let Some(event) = window_event {
        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                window = Some(match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(*x, *y);

                        GameWindowEvent::MouseWheel { delta }
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(delta.x as Num, delta.y as Num);

                        GameWindowEvent::MouseWheel { delta }
                    }
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        window = Some(if *state == ElementState::Pressed {
                            GameWindowEvent::MainPressed
                        } else {
                            GameWindowEvent::MainReleased
                        });
                    }
                    _ => (),
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                window = Some(GameWindowEvent::ModifierChanged {
                    modifier: *modifier,
                });
            }
            WindowEvent::CursorMoved { position, .. } => {
                window = Some(GameWindowEvent::CursorPos {
                    pos: point2(position.x as f32, position.y as f32),
                });
            }
            _ => (),
        }
    }

    if let Some(event) = &device_event {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let delta = vec2(delta.0 as Num, delta.1 as Num);

                device = Some(GameDeviceEvent::MainMove { delta });
            }
            _ => (),
        }
    }

    GameInputEvent(window, device)
}

impl Handler<GameInputEvent> for InputHandler {
    type Result = Option<()>;

    fn handle(&mut self, input: GameInputEvent, _ctx: &mut Self::Context) -> Self::Result {
        let mut state = InputState::default();

        let (window_event, device_event) = (input.0, input.1);

        if let Some(event) = window_event {
            match event {
                GameWindowEvent::MainPressed => {
                    self.main_clicked = true;
                }
                GameWindowEvent::MainReleased => {
                    self.main_clicked = false;
                }
                GameWindowEvent::MouseWheel { delta } => {
                    state.scroll = Some(delta);
                }
                GameWindowEvent::ModifierChanged { modifier } => {
                    self.modifier = modifier;
                }
                GameWindowEvent::CursorPos { pos } => {
                    self.cursor_pos = pos;
                }
            }
        }

        if self.modifier.shift() {
            state.modifier_shift = true;
        }

        if let Some(event) = device_event {
            match event {
                GameDeviceEvent::MainMove { delta } => {
                    state.main_move = Some(delta);
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
