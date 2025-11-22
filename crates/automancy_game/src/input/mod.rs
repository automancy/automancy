pub mod camera;

use std::cell::Cell;

use automancy_data::{
    id::Id,
    math::{Float, Vec2},
};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use winit::{
    event::{DeviceEvent, ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, NamedKey, SmolStr},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::{persistent::options::GameOptions, resources::ResourceManager};

thread_local! {
    static DEFAULT_KEYMAP: Cell<Option<HashMap<Key, KeyAction>>> = Cell::default();
}

fn set_default_keymap(resource_man: &ResourceManager) {
    let cancel: KeyAction = KeyAction {
        ty: ActionType::Cancel,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.cancel),
    };
    let undo: KeyAction = KeyAction {
        ty: ActionType::Undo,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.undo),
    };
    let redo: KeyAction = KeyAction {
        ty: ActionType::Redo,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.redo),
    };
    let debug: KeyAction = KeyAction {
        ty: ActionType::Debug,
        press_type: PressType::Toggle,
        name: None,
    };
    let fullscreen: KeyAction = KeyAction {
        ty: ActionType::Fullscreen,
        press_type: PressType::Tap,
        name: None,
    };
    let screenshot: KeyAction = KeyAction {
        ty: ActionType::Screenshot,
        press_type: PressType::Tap,
        name: None,
    };
    let toggle_gui: KeyAction = KeyAction {
        ty: ActionType::ToggleGui,
        press_type: PressType::Toggle,
        name: Some(resource_man.registry.key_ids.toggle_gui),
    };
    let player: KeyAction = KeyAction {
        ty: ActionType::Player,
        press_type: PressType::Toggle,
        name: Some(resource_man.registry.key_ids.player_menu),
    };
    let delete: KeyAction = KeyAction {
        ty: ActionType::Delete,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.remove_tile),
    };
    let select_mode: KeyAction = KeyAction {
        ty: ActionType::SelectMode,
        press_type: PressType::Hold,
        name: Some(resource_man.registry.key_ids.select_mode),
    };
    let hotkey: KeyAction = KeyAction {
        ty: ActionType::HotkeyActive,
        press_type: PressType::Hold,
        name: Some(resource_man.registry.key_ids.hotkey),
    };
    let cut: KeyAction = KeyAction {
        ty: ActionType::Cut,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.cut),
    };
    let copy: KeyAction = KeyAction {
        ty: ActionType::Copy,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.copy),
    };
    let paste: KeyAction = KeyAction {
        ty: ActionType::Paste,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.paste),
    };

    DEFAULT_KEYMAP.set(Some(HashMap::from_iter([
        (Key::Character(SmolStr::new_inline("z")), undo),
        (Key::Character(SmolStr::new_inline("r")), redo),
        (Key::Character(SmolStr::new_inline("e")), player),
        (Key::Character(SmolStr::new_inline("x")), cut),
        (Key::Character(SmolStr::new_inline("c")), copy),
        (Key::Character(SmolStr::new_inline("v")), paste),
        (Key::Named(NamedKey::Escape), cancel),
        (Key::Named(NamedKey::F1), toggle_gui),
        (Key::Named(NamedKey::F2), screenshot),
        (Key::Named(NamedKey::F3), debug),
        (Key::Named(NamedKey::F11), fullscreen),
        (Key::Named(NamedKey::Backspace), delete),
        (Key::Named(NamedKey::Shift), select_mode),
        (Key::Named(NamedKey::Control), hotkey),
    ])));
}

pub fn get_default_keymap(resource_man: &ResourceManager) -> HashMap<Key, KeyAction> {
    let taken = DEFAULT_KEYMAP.take();

    if let Some(taken) = taken {
        DEFAULT_KEYMAP.set(Some(taken.clone()));

        taken
    } else {
        set_default_keymap(resource_man);

        get_default_keymap(resource_man)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ActionType {
    Cancel,
    Undo,
    Redo,
    Debug,
    Fullscreen,
    Screenshot,
    ToggleGui,
    Player,
    Delete,
    SelectMode,
    HotkeyActive,
    Cut,
    Copy,
    Paste,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PressType {
    Tap,    // returns true when the key is pressed once and will not press again until released
    Hold,   // returns true whenever the key is down
    Toggle, // pressing the key will either toggle it on or off
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct KeyAction {
    pub ty: ActionType,
    pub press_type: PressType,
    #[serde(skip)]
    pub name: Option<Id>,
}

/// The various controls of the game.
#[derive(Debug, Clone)]
pub enum GameInputEvent {
    None,
    MainPos { pos: Vec2 },
    MainMove { delta: Vec2 },
    MouseWheel { delta: Vec2 },
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

impl GameInputEvent {
    #[allow(clippy::single_match)]
    #[allow(clippy::collapsible_match)]
    pub fn from_winit_event(
        window_event: Option<&WindowEvent>,
        device_event: Option<&DeviceEvent>,
        viewport_size: Vec2,
        sensitivity: Float,
    ) -> GameInputEvent {
        let mut result = GameInputEvent::None;

        if let Some(event) = window_event {
            match event {
                WindowEvent::MouseWheel { delta, .. } => {
                    result = match delta {
                        MouseScrollDelta::PixelDelta(delta) => {
                            let delta = Vec2::new(
                                (delta.x as f32 / viewport_size.x) * sensitivity,
                                (delta.y as f32 / viewport_size.y) * sensitivity,
                            );

                            GameInputEvent::MouseWheel { delta }
                        }
                        MouseScrollDelta::LineDelta(x, y) => {
                            // Observed logical pixels per scroll wheel increment in Windows on Chrome
                            // (copied from yakui)
                            const LINE_HEIGHT: f32 = 100.0 / 3.0;

                            let delta = Vec2::new((*x / LINE_HEIGHT) * sensitivity, (*y / LINE_HEIGHT) * sensitivity);

                            GameInputEvent::MouseWheel { delta }
                        }
                    };
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    match button {
                        MouseButton::Left => {
                            result = if state == &ElementState::Pressed {
                                GameInputEvent::MainPressed
                            } else {
                                GameInputEvent::MainReleased
                            };
                        }
                        MouseButton::Right => {
                            result = if state == &ElementState::Pressed {
                                GameInputEvent::AlternatePressed
                            } else {
                                GameInputEvent::AlternateReleased
                            };
                        }
                        MouseButton::Middle => {
                            result = if state == &ElementState::Pressed {
                                GameInputEvent::TertiaryPressed
                            } else {
                                GameInputEvent::TertiaryReleased
                            }
                        }
                        _ => {}
                    };
                }
                WindowEvent::ModifiersChanged(modifier) => {
                    result = GameInputEvent::ModifierChanged { modifier: *modifier };
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Vec2::new(position.x as Float, position.y as Float);

                    result = GameInputEvent::MainPos { pos };
                }
                WindowEvent::KeyboardInput { event, .. } => result = GameInputEvent::KeyboardEvent { event: event.clone() },
                _ => {}
            }
        }

        if let Some(event) = device_event {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    let delta = Vec2::new(delta.0 as Float * sensitivity, -delta.1 as Float * sensitivity);

                    result = GameInputEvent::MainMove { delta };
                }
                _ => {}
            }
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct InputHandler {
    pub main_pos: Vec2,
    pub scroll: Option<Vec2>,
    pub main_move: Option<Vec2>,

    pub main_held: bool,
    pub alternate_held: bool,
    pub tertiary_held: bool,

    pub main_pressed: bool,
    pub alternate_pressed: bool,
    pub tertiary_pressed: bool,

    pub key_map: HashMap<Key, KeyAction>,
    pub key_states: HashSet<ActionType>,

    to_clear: Vec<KeyAction>,
}

impl InputHandler {
    pub fn new(options: &GameOptions) -> Self {
        Self {
            main_pos: Vec2::new(0.0, 0.0),
            scroll: None,
            main_move: None,

            main_held: false,
            alternate_held: false,
            tertiary_held: false,

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

        for v in std::mem::take(&mut self.to_clear) {
            self.key_states.remove(&v.ty);
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
            GameInputEvent::KeyboardEvent { event } => {
                self.handle_key(event.state, event.key_without_modifiers());
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, state: ElementState, key: Key) -> Option<()> {
        let action = *self.key_map.get(&key)?;

        match action.press_type {
            PressType::Tap => match state {
                ElementState::Pressed => {
                    self.key_states.insert(action.ty);
                    self.to_clear.push(action);
                }
                ElementState::Released => {
                    self.key_states.remove(&action.ty);
                }
            },
            PressType::Hold => match state {
                ElementState::Pressed => {
                    self.key_states.insert(action.ty);
                }
                ElementState::Released => {
                    self.key_states.remove(&action.ty);
                }
            },
            PressType::Toggle => match state {
                ElementState::Pressed => {
                    if self.key_states.contains(&action.ty) {
                        self.key_states.remove(&action.ty);
                    } else {
                        self.key_states.insert(action.ty);
                    }
                }
                ElementState::Released => {}
            },
        }

        Some(())
    }

    pub fn key_active(&self, action: ActionType) -> bool {
        self.key_states.contains(&action)
    }
}
