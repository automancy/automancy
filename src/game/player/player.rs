use std::{cell::RefCell, rc::Rc};

use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};

use crate::{game::render::camera::Camera, math::data::Point3};

use super::control::{MainClickListener, MainHoldListener, MainMoveListener};

use paste::paste;
pub struct Player {
    pub pos: Rc<RefCell<Point3>>,
    pub camera: Rc<RefCell<Camera>>,

    main_clicked: bool,
    main_last_clicked: u32,

    main_hold_listeners: Vec<Rc<RefCell<dyn MainHoldListener>>>,
    main_click_listeners: Vec<Rc<RefCell<dyn MainClickListener>>>,
    main_move_listeners: Vec<Rc<RefCell<dyn MainMoveListener>>>,
}

macro_rules! listener {
    ($name: ident, $Type: ty) => {
        paste! {
            pub fn [< register_ $name >](&mut self, listener: Rc<RefCell<dyn $Type>>) {
                self.[< $name _listeners >].push(listener);
                self.[< $name _listeners >].sort_by(|a, b| a.borrow().partial_cmp(&b.borrow().id()).unwrap_or(std::cmp::Ordering::Less));
            }
        }
    };
}

impl Player {
    listener!(main_hold, MainHoldListener);
    listener!(main_click, MainClickListener);
    listener!(main_move, MainMoveListener);

    pub fn new(pos: Rc<RefCell<Point3>>, camera: Rc<RefCell<Camera>>) -> Self {
        let mut result = Player {
            pos,
            camera,
            main_clicked: false,
            main_last_clicked: 0,
            main_hold_listeners: vec![],
            main_click_listeners: vec![],
            main_move_listeners: vec![],
        };

        result.register_main_hold(result.camera.clone());
        result.register_main_move(result.camera.clone());

        result
    }

    pub fn handle_events(
        &mut self,
        window_event: Option<WindowEvent>,
        device_event: Option<DeviceEvent>,
    ) {
        let mut cursor_position = None;

        if let Some(event) = window_event {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {}
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::MouseWheel { delta, phase, .. } => {}
                WindowEvent::MouseInput { state, button, .. } => {
                    match button {
                        MouseButton::Left => {
                            self.main_clicked = state == ElementState::Pressed;
                        }
                        _ => (),
                    };
                }
                _ => (),
            }
        }

        if let Some(event) = device_event {
            match event {
                DeviceEvent::MouseMotion { delta } => cursor_position = Some(delta),
                _ => (),
            }
        }

        if self.main_clicked {
            self.main_click_listeners
                .iter_mut()
                .for_each(|v| v.borrow_mut().on_clicking_main());
        }

        if self.main_last_clicked > 0 {
            let elapsed = (self.main_last_clicked as f32) / 60.0; // TODO get FPS

            self.main_hold_listeners
                .iter_mut()
                .for_each(|v| v.borrow_mut().on_holding_main(elapsed));
        }

        if let Some(delta) = cursor_position {
            self.main_move_listeners
                .iter_mut()
                .for_each(|v| v.borrow_mut().on_moving_main(delta));
        }

        if self.main_clicked {
            self.main_last_clicked += 1;
        } else {
            self.main_last_clicked = 0;

            self.main_hold_listeners
                .iter_mut()
                .for_each(|v| v.borrow_mut().on_not_holding_main());
        }
    }
}
