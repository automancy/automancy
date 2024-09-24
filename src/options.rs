use std::{
    fs::{read_to_string, File},
    path::Path,
};
use std::{io::Write, mem};

use automancy_resources::ResourceManager;
use hashbrown::HashMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use winit::keyboard::Key;

use crate::input::{get_default_keymap, KeyAction};

static OPTIONS_PATH: &str = "options.ron";
static MISC_OPTIONS_PATH: &str = "misc_options.ron";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscOptions {
    pub language: String,

    #[serde(skip)]
    pub synced: bool,
}

impl Default for MiscOptions {
    fn default() -> Self {
        Self {
            language: String::from("en_US"),
            synced: false,
        }
    }
}

impl MiscOptions {
    pub fn load() -> Self {
        log::info!("Loading options...");

        let file = read_to_string(Path::new(MISC_OPTIONS_PATH)).unwrap_or_default();

        let mut this: MiscOptions = ron::de::from_str(&file)
            .inspect_err(|err| {
                log::warn!("Error parsing misc options! A fresh one will be created. Error: {err}")
            })
            .unwrap_or_default();

        if let Err(err) = this.save() {
            log::error!("Error saving misc options! {err}");
        }

        this
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut file = File::create(MISC_OPTIONS_PATH)?;

        log::info!("Serializing misc options...");
        log::debug!("{self:?}");

        let document = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .inspect_err(|err| log::warn!("Error writing misc options! Error: {err}"))?;

        log::info!("Saving misc options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved misc options!");

        self.synced = false;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub gui: GuiOptions,
    pub keymap: HashMap<Key, KeyAction>,

    #[serde(skip)]
    pub synced: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Options {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            audio: Default::default(),
            gui: Default::default(),
            keymap: Default::default(),
            synced: false,
        }
    }
}

impl Options {
    pub fn load(resource_man: &ResourceManager) -> Self {
        log::info!("Loading options...");

        let file = read_to_string(Path::new(OPTIONS_PATH)).unwrap_or_default();

        let mut this: Options = ron::de::from_str(&file)
            .inspect_err(|err| {
                log::warn!("Error parsing options! A fresh one will be created. Error: {err}")
            })
            .unwrap_or_default();
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

        let document = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .inspect_err(|err| log::warn!("Error writing options! Error: {err}"))?;

        log::info!("Saving options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved options!");

        self.synced = false;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AAType {
    None,
    FXAA,
    TAA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiScale {
    Small,
    Normal,
    Large,
}

impl UiScale {
    pub const fn to_f64(self) -> f64 {
        match self {
            UiScale::Small => const { 2.0 / 3.0 },
            UiScale::Normal => const { 1.0 },
            UiScale::Large => const { 5.0 / 3.0 },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GraphicsOptions {
    pub fps_limit: i32,
    pub fullscreen: bool,
    pub ui_scale: UiScale,
    pub anti_aliasing: AAType,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            fps_limit: 0,
            fullscreen: false,
            ui_scale: UiScale::Normal,
            anti_aliasing: AAType::FXAA,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GuiOptions {
    font: Option<String>,
}

impl GuiOptions {
    pub fn get_font(&self, resource_man: &ResourceManager) -> Option<String> {
        self.font
            .clone()
            .or_else(|| resource_man.fonts.keys().next().cloned())
    }

    pub fn set_font(&mut self, resource_man: &ResourceManager, font: Option<String>) {
        if font
            .as_ref()
            .is_some_and(|font| resource_man.fonts.contains_key(font))
        {
            self.font = font
        } else {
            self.font = None
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
