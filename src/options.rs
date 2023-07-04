use enum_ordinalize::Ordinalize;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use winit::event::VirtualKeyCode;

use automancy_defs::hashbrown::HashMap;
use automancy_defs::log;

use crate::input::{actions, KeyAction};

#[derive(Serialize, Deserialize)]
pub struct Options {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub keymap: HashMap<VirtualKeyCode, KeyAction>,
}
lazy_static! {
    pub static ref DEFAULT_KEYMAP: HashMap<VirtualKeyCode, KeyAction> = HashMap::from([
        (VirtualKeyCode::Z, actions::UNDO),
        (VirtualKeyCode::Escape, actions::ESCAPE),
        (VirtualKeyCode::F3, actions::DEBUG),
        (VirtualKeyCode::F11, actions::FULLSCREEN)
    ]);
}
impl Default for Options {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            audio: Default::default(),
            keymap: DEFAULT_KEYMAP.clone(),
        }
    }
}

static OPTIONS_PATH: &str = "options.toml";

impl Options {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        log::info!("Loading options...");
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(OPTIONS_PATH)?;
        let mut body = String::new();
        BufReader::new(file).read_to_string(&mut body)?;
        let mut this: Self = toml::de::from_str(body.clone().as_str()).unwrap_or_default();

        if this.keymap.len() != DEFAULT_KEYMAP.len() {
            // TODO show a popup warning the player
            this.keymap = DEFAULT_KEYMAP.clone();
        }

        this.save()?;

        Ok(this)
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(OPTIONS_PATH)?;

        let body = toml::ser::to_string(self)?;
        write!(&mut file, "{body}")?;

        log::info!("Saved options!");

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Ordinalize)]
pub enum AALevel {
    None,
    FXAA,
    TAA,
    MSAA,
}
#[derive(Serialize, Deserialize)]
pub struct GraphicsOptions {
    pub fps_limit: f64,
    pub fullscreen: bool,
    pub scale: f32,
    pub aa: AALevel,
}
impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            fps_limit: 0.0,
            fullscreen: false,
            scale: 1.0,
            aa: AALevel::MSAA,
        }
    }
}
#[derive(Serialize, Deserialize, Default)]
pub struct AudioOptions {
    pub sfx_volume: f64,
    pub music_volume: f64,
}
