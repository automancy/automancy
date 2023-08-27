use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};

use enum_ordinalize::Ordinalize;
use serde::{Deserialize, Serialize};
use winit::event::VirtualKeyCode;

use automancy_defs::hashbrown::HashMap;
use automancy_defs::log;
use automancy_defs::math::{Double, Float};

use crate::input::{KeyAction, DEFAULT_KEYMAP};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub keymap: HashMap<VirtualKeyCode, KeyAction>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            audio: Default::default(),
            keymap: DEFAULT_KEYMAP.iter().cloned().collect(),
        }
    }
}

static OPTIONS_PATH: &str = "options.toml";

impl Options {
    pub fn load() -> anyhow::Result<Options> {
        log::info!("Loading options...");

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(OPTIONS_PATH)?;
        let mut body = String::new();

        BufReader::new(file).read_to_string(&mut body)?;
        let mut this: Options = toml::de::from_str(body.clone().as_str()).unwrap_or_default();

        if this.keymap.len() != DEFAULT_KEYMAP.len() {
            // TODO show a popup warning the player
            this.keymap = DEFAULT_KEYMAP.iter().cloned().collect();
        }

        this.save()?;

        Ok(this)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut file = File::create(OPTIONS_PATH)?;

        let document = toml::ser::to_string_pretty(&self)?;

        write!(&mut file, "{document}")?;

        log::info!("Saved options!");

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ordinalize)]
pub enum AAType {
    None,
    FXAA,
    TAA,
    Upscale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GraphicsOptions {
    pub fps_limit: Double,
    pub fullscreen: bool,
    pub scale: Float,
    pub anti_aliasing: AAType,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            fps_limit: 0.0,
            fullscreen: false,
            scale: 1.0,
            anti_aliasing: AAType::Upscale,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioOptions {
    pub sfx_volume: f64,
    pub music_volume: f64,
}

impl Default for AudioOptions {
    fn default() -> Self {
        Self {
            sfx_volume: 0.5,
            music_volume: 0.5,
        }
    }
}
