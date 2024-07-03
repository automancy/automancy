use std::{
    fs::{read_to_string, File},
    path::Path,
};
use std::{io::Write, mem};

use automancy_resources::ResourceManager;
use enum_ordinalize::Ordinalize;
use hashbrown::HashMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use winit::keyboard::Key;

use automancy_defs::log;
use automancy_defs::math::Double;

use crate::input::{get_default_keymap, KeyAction};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Options {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub gui: GuiOptions,
    pub keymap: HashMap<Key, KeyAction>,
    pub synced: bool,
}

impl Options {
    pub fn new() -> Self {
        Self {
            graphics: Default::default(),
            audio: Default::default(),
            gui: Default::default(),
            keymap: Default::default(),
            synced: false,
        }
    }
}

static OPTIONS_PATH: &str = "options.ron";

impl Options {
    pub fn load(resource_man: &ResourceManager) -> Options {
        log::info!("Loading options...");

        let file = read_to_string(Path::new(OPTIONS_PATH)).unwrap_or_default();

        let mut this: Options = ron::de::from_str(&file).unwrap_or_default();
        let read_keymap = mem::take(&mut this.keymap);

        let mut default = get_default_keymap(resource_man);
        for (key, read_action) in read_keymap {
            let mut modified_action = default[&key];
            modified_action.action = read_action.action;

            default.insert(key, modified_action);
        }

        for original in &default {
            if let Some(other) = default
                .iter()
                .find(|other| original.0 != other.0 && original.1.action == other.1.action)
            {
                log::error!(
                    "Action {:?} has multiple bound keys! First: {:?}, second: {:?}. Resetting keymap.",
                    original.1.action,
                    original.0,
                    other.0
                );
                default = get_default_keymap(resource_man);
                break;
            }
        }

        this.keymap = default;

        if let Err(err) = this.save() {
            log::error!("Error saving options! {err}");
        }

        this
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
    pub fps_limit: i32,
    pub fullscreen: bool,
    pub scale: Double,
    pub anti_aliasing: AAType,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            fps_limit: 0,
            fullscreen: false,
            scale: 1.0,
            anti_aliasing: AAType::FXAA,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GuiOptions {
    pub font: Option<String>,
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
