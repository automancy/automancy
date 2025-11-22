use std::{
    fs::{File, read_to_string},
    io::Write,
    path::Path,
};

use automancy_data::math::UInt;
use hashbrown::HashMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use winit::keyboard::Key;

use crate::{
    input::{KeyAction, get_default_keymap},
    persistent,
    resources::ResourceManager,
};

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

        let mut this: MiscOptions = persistent::ron::ron_options()
            .from_str(&file)
            .inspect_err(|err| log::warn!("Error parsing misc options! A fresh one will be created. Error: {err}"))
            .unwrap_or_default();

        if let Err(err) = this.save() {
            log::error!("Error saving misc options! {err}");
        }

        this
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut file = File::create(MISC_OPTIONS_PATH)?;

        log::info!("Serializing misc options...");
        log::debug!("\n{self:?}");

        let document = persistent::ron::ron_options()
            .to_string_pretty(&self, PrettyConfig::default())
            .inspect_err(|err| log::warn!("Error writing misc options! Error: {err}"))?;

        log::info!("Saving misc options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved misc options!");

        self.synced = false;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOptions {
    pub graphics: GraphicsOptions,
    pub audio: AudioOptions,
    pub gui: GuiOptions,
    pub keymap: HashMap<Key, KeyAction>,

    #[serde(skip)]
    pub synced: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for GameOptions {
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

impl GameOptions {
    pub fn load(resource_man: &ResourceManager) -> Self {
        log::info!("Loading options...");

        let file = read_to_string(Path::new(OPTIONS_PATH)).unwrap_or_default();

        let mut this: GameOptions = persistent::ron::ron_options()
            .from_str(&file)
            .inspect_err(|err| log::warn!("Error parsing options! A fresh one will be created. Error: {err}"))
            .unwrap_or_default();

        let mut keymap = get_default_keymap(resource_man);
        for (key, read_action) in std::mem::take(&mut this.keymap) {
            let mut modified_action = keymap[&key];
            modified_action.ty = read_action.ty;

            keymap.insert(key, modified_action);
        }
        this.keymap = keymap;

        for (key, action) in &this.keymap {
            if let Some((other_key, _)) = this
                .keymap
                .iter()
                .find(|(other_key, other_action)| action.ty == other_action.ty && key != *other_key)
            {
                log::error!(
                    "Action {:?} has multiple bound keys! First: {:?}, second: {:?}. Resetting keymap.",
                    action,
                    key,
                    other_key
                );
                this.keymap = get_default_keymap(resource_man);
                break;
            }
        }

        if let Err(err) = this.save() {
            log::error!("Error saving options! {err}");
        }

        this
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut file = File::create(OPTIONS_PATH)?;

        log::info!("Serializing options...");
        log::debug!("\n{self:?}");

        let document = persistent::ron::ron_options()
            .to_string_pretty(&self, PrettyConfig::default())
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
    pub fps_limit: UInt,
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
    #[serde(default)]
    font: Option<String>,
}

impl GuiOptions {
    pub fn get_font(&self, resource_man: &ResourceManager) -> Option<String> {
        self.font.clone().or_else(|| resource_man.fonts.keys().next().cloned())
    }

    pub fn set_font(&mut self, resource_man: &ResourceManager, font: Option<String>) {
        if font.as_ref().is_some_and(|font| resource_man.fonts.contains_key(font)) {
            self.font = font
        } else {
            self.font = None
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AudioOptions {
    pub sfx_volume: f32,
    pub music_volume: f32,
}

impl Default for AudioOptions {
    fn default() -> Self {
        Self {
            sfx_volume: 0.5,
            music_volume: 0.5,
        }
    }
}
