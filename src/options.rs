use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};

use enum_ordinalize::Ordinalize;
use hashbrown::HashMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use winit::keyboard::Key;

use automancy_defs::log;
use automancy_defs::math::{Double, Float};

use crate::input::{KeyAction, DEFAULT_KEYMAP};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub gui: GuiOptions,
    pub keymap: HashMap<Key, KeyAction>,
    pub synced: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            audio: Default::default(),
            gui: Default::default(),
            keymap: DEFAULT_KEYMAP.iter().cloned().collect(),
            synced: false,
        }
    }
}

static OPTIONS_PATH: &str = "options.ron";

impl Options {
    pub fn load() -> anyhow::Result<Options> {
        log::info!("Loading options...");

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(OPTIONS_PATH)?;

        let reader = BufReader::new(file);

        let mut this: Options = ron::de::from_reader(reader).unwrap_or_default();

        if this.keymap.len() != DEFAULT_KEYMAP.len() {
            // TODO show a popup warning the player
            this.keymap = DEFAULT_KEYMAP.iter().cloned().collect();
        }

        this.save()?;

        Ok(this)
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut file = File::create(OPTIONS_PATH)?;

        log::info!("Serializing options...");
        log::debug!("{self:?}");

        let document = ron::ser::to_string_pretty(&self, PrettyConfig::default())?;

        log::info!("Saving options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved options!");

        self.synced = false;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ordinalize)]
pub enum AAType {
    None,
    FXAA,
    TAA,
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
            anti_aliasing: AAType::FXAA,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiOptions {
    pub scale: f32,
    pub font: String,
}

impl Default for GuiOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            font: "iosevka-extended.ttf".to_string(),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
