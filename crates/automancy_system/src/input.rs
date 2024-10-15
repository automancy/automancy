use crate::options::GameOptions;
use automancy_defs::id::Id;
use automancy_defs::{
    glam::vec2,
    math::{Float, Vec2},
};
use automancy_resources::ResourceManager;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, mem};
use winit::event::{
    DeviceEvent, ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::keyboard::{Key, NamedKey, SmolStr};
use winit::{
    event::ElementState::{Pressed, Released},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

thread_local! {
    static DEFAULT_KEYMAP: Cell<Option<HashMap<Key, KeyAction>>> = Cell::default();
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

fn set_default_keymap(resource_man: &ResourceManager) {
    let cancel: KeyAction = KeyAction {
        action: ActionType::Cancel,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.cancel),
    };
    let undo: KeyAction = KeyAction {
        action: ActionType::Undo,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.undo),
    };
    let redo: KeyAction = KeyAction {
        action: ActionType::Redo,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.redo),
    };
    let debug: KeyAction = KeyAction {
        action: ActionType::Debug,
        press_type: PressType::Toggle,
        name: None,
    };
    let fullscreen: KeyAction = KeyAction {
        action: ActionType::Fullscreen,
        press_type: PressType::Tap,
        name: None,
    };
    let screenshot: KeyAction = KeyAction {
        action: ActionType::Screenshot,
        press_type: PressType::Tap,
        name: None,
    };
    let toggle_gui: KeyAction = KeyAction {
        action: ActionType::ToggleGui,
        press_type: PressType::Toggle,
        name: Some(resource_man.registry.key_ids.toggle_gui),
    };
    let player: KeyAction = KeyAction {
        action: ActionType::Player,
        press_type: PressType::Toggle,
        name: Some(resource_man.registry.key_ids.player_menu),
    };
    let delete: KeyAction = KeyAction {
        action: ActionType::Delete,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.remove_tile),
    };
    let select_mode: KeyAction = KeyAction {
        action: ActionType::SelectMode,
        press_type: PressType::Hold,
        name: Some(resource_man.registry.key_ids.select_mode),
    };
    let hotkey: KeyAction = KeyAction {
        action: ActionType::HotkeyActive,
        press_type: PressType::Hold,
        name: Some(resource_man.registry.key_ids.hotkey),
    };
    let cut: KeyAction = KeyAction {
        action: ActionType::Cut,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.cut),
    };
    let copy: KeyAction = KeyAction {
        action: ActionType::Copy,
        press_type: PressType::Tap,
        name: Some(resource_man.registry.key_ids.copy),
    };
    let paste: KeyAction = KeyAction {
        action: ActionType::Paste,
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
    pub action: ActionType,
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

pub fn convert_input(
    window_event: Option<&WindowEvent>,
    device_event: Option<&DeviceEvent>,
    (width, height): (Float, Float),
    sensitivity: Float,
) -> GameInputEvent {
    let mut result = GameInputEvent::None;

    if let Some(event) = window_event {
        use GameInputEvent::*;

        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                result = match delta {
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = vec2(
                            delta.x as f32 / width * sensitivity,
                            delta.y as f32 / height * sensitivity,
                        );

                        MouseWheel { delta }
                    }
                    MouseScrollDelta::LineDelta(x, y) => {
                        let delta = vec2(*x * sensitivity, *y * sensitivity);

                        MouseWheel { delta }
                    }
                };
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        result = if state == &Pressed {
                            MainPressed
                        } else {
                            MainReleased
                        };
                    }
                    MouseButton::Right => {
                        result = if state == &Pressed {
                            AlternatePressed
                        } else {
                            AlternateReleased
                        };
                    }
                    MouseButton::Middle => {
                        result = if state == &Pressed {
                            TertiaryPressed
                        } else {
                            TertiaryReleased
                        }
                    }
                    _ => {}
                };
            }
            WindowEvent::ModifiersChanged(modifier) => {
                result = ModifierChanged {
                    modifier: *modifier,
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = vec2(position.x as Float, position.y as Float);

                result = MainPos { pos };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                result = KeyboardEvent {
                    event: event.clone(),
                }
            }
            _ => {}
        }
    }

    if let Some(event) = device_event {
        use GameInputEvent::*;

        if let DeviceEvent::MouseMotion { delta } = event {
            let delta = vec2(
                delta.0 as Float * sensitivity,
                -delta.1 as Float * sensitivity,
            );

            result = MainMove { delta };
        }
    }

    result
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
            main_pos: vec2(0.0, 0.0),
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

        for v in mem::take(&mut self.to_clear) {
            self.key_states.remove(&v.action);
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
                Pressed => {
                    self.key_states.insert(action.action);
                    self.to_clear.push(action);
                }
                Released => {
                    self.key_states.remove(&action.action);
                }
            },
            PressType::Hold => match state {
                Pressed => {
                    self.key_states.insert(action.action);
                }
                Released => {
                    self.key_states.remove(&action.action);
                }
            },
            PressType::Toggle => match state {
                Pressed => {
                    if self.key_states.contains(&action.action) {
                        self.key_states.remove(&action.action);
                    } else {
                        self.key_states.insert(action.action);
                    }
                }
                Released => {}
            },
        }

        Some(())
    }

    pub fn key_active(&self, action: ActionType) -> bool {
        self.key_states.contains(&action)
    }
}
