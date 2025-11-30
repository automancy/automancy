use std::{
    fs::{File, read_to_string},
    io::Write,
    path::Path,
};

use automancy_data::math::UInt;
use hashbrown::HashMap;
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

impl PartialEq for MiscOptions {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language
    }
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
        log::info!("Loading misc options...");

        let file = read_to_string(Path::new(MISC_OPTIONS_PATH)).unwrap_or_default();

        let mut this: MiscOptions = persistent::ron::ron_options()
            .from_str(&file)
            .inspect_err(|err| log::warn!("Error parsing misc options! A fresh config will be created. Error: {err}"))
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

        let document =
            persistent::ron::to_string_pretty(&self).inspect_err(|err| log::warn!("Error serializing misc options! Error: {err}"))?;

        log::info!("Saving misc options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved misc options!");

        Ok(())
    }

    pub fn mark_out_of_sync(&mut self) {
        self.synced = false;
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

impl PartialEq for GameOptions {
    fn eq(&self, other: &Self) -> bool {
        self.graphics == other.graphics && self.audio == other.audio && self.gui == other.gui && self.keymap == other.keymap
    }
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
        log::info!("Loading game options...");

        let file = read_to_string(Path::new(OPTIONS_PATH)).unwrap_or_default();

        let mut this: GameOptions = persistent::ron::ron_options()
            .from_str(&file)
            .inspect_err(|err| log::warn!("Error parsing game options! A fresh config will be created. Error: {err}"))
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
            log::error!("Error saving game options! {err}");
        }

        this
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.graphics.sanitize();

        let mut file = File::create(OPTIONS_PATH)?;

        log::info!("Serializing game options...");
        log::debug!("\n{self:?}");

        let document =
            persistent::ron::to_string_pretty(&self).inspect_err(|err| log::warn!("Error serializing game options! Error: {err}"))?;

        log::info!("Saving game options...");

        write!(&mut file, "{document}")?;

        log::info!("Saved game options!");

        Ok(())
    }

    pub fn mark_out_of_sync(&mut self) {
        self.synced = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AAType {
    None,
    FXAA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiScale {
    Small,
    Normal,
    Large,
}

impl UiScale {
    pub const fn to_f32(self) -> f32 {
        match self {
            UiScale::Small => 0.8,
            UiScale::Normal => 1.0,
            UiScale::Large => 1.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphicsOptions {
    fps_limit: UInt,
    pub fullscreen: bool,
    pub ui_scale: UiScale,
    pub antialiasing_type: AAType,
}

impl Default for GraphicsOptions {
    fn default() -> Self {
        Self {
            fps_limit: 0,
            fullscreen: false,
            ui_scale: UiScale::Normal,
            antialiasing_type: AAType::FXAA,
        }
    }
}

impl GraphicsOptions {
    fn sanitize(&mut self) {
        self.clamp_fps_limit();
    }

    fn clamp_fps_limit(&mut self) {
        if self.fps_limit >= 15 && self.fps_limit <= 30 {
            self.fps_limit = 30;
        }
        if self.fps_limit < 15 {
            self.fps_limit = 0;
        }
    }

    pub fn set_fps_limit(&mut self, v: UInt) {
        self.fps_limit = v;
        self.clamp_fps_limit();
    }

    pub fn fps_limit(&self) -> UInt {
        self.fps_limit
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GuiOptions {
    #[serde(default)]
    font: Option<String>,
    system_fonts: bool,
    system_fonts_preferences: bool,
}

impl GuiOptions {
    pub fn set_system_fonts(&mut self, v: bool) {
        self.system_fonts = v;

        if !self.system_fonts {
            self.system_fonts_preferences = false;
        }
    }

    pub fn system_fonts(&self) -> bool {
        self.system_fonts
    }

    pub fn set_system_fonts_preferences(&mut self, v: bool) {
        self.system_fonts_preferences = v;

        if self.system_fonts_preferences {
            self.system_fonts = true;
        }
    }

    pub fn system_fonts_preferences(&self) -> bool {
        self.system_fonts_preferences
    }

    pub fn set_font(&mut self, font: Option<String>) {
        self.font = font
    }

    pub fn font(&self) -> Option<&str> {
        self.font.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
