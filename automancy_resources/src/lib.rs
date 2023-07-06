use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub use chrono;
use chrono::{DateTime, Local};
pub use kira;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use walkdir::WalkDir;

use automancy_defs::flexstr::SharedStr;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_defs::rendering::Mesh;

use crate::error::ErrorManager;
use crate::model::IndexRange;
use crate::registry::{DataIds, ErrorIds, GuiIds, ModelIds, Registry, TileIds};
use crate::translate::Translate;

pub mod audio;
pub mod data;
pub mod error;
pub mod item;
pub mod model;
pub mod registry;
pub mod script;
pub mod shader;
pub mod tag;
pub mod tile;
pub mod translate;

// TODO this fucking sucks
/// like format!, but does not require the format string to be static.
pub fn format(format: &str, args: &[&str]) -> String {
    let mut string = format.to_string();
    for arg in args {
        string = string.replacen("{}", arg, 1);
    }
    string
}

/// Converts a UTC Unix timestamp into a formatted time string, using the given strftime format string.
pub fn format_time(time: SystemTime, fmt: &str) -> String {
    let time = DateTime::<Local>::from(time);
    time.format(fmt).to_string()
}

pub fn load_recursively(path: &Path, extension: &OsStr) -> Vec<PathBuf> {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|v| v.path().extension() == Some(extension))
        .map(|v| v.path().to_path_buf())
        .collect()
}

pub static RESOURCES_PATH: &str = "resources";

pub static JSON_EXT: &str = "json";
pub static AUDIO_EXT: &str = "ogg";
pub static SHADER_EXT: &str = "wgsl";
pub static IMAGE_EXT: &str = "png";

/// TODO set of suffixes

/// Represents a resource manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,
    pub error_man: ErrorManager,

    pub registry: Registry,

    pub translates: Translate,
    pub audio: HashMap<SharedStr, StaticSoundData>,
    pub shaders: HashMap<SharedStr, String>,

    pub ordered_tiles: Vec<Id>,
    pub ordered_items: Vec<Id>,
    pub index_ranges: HashMap<Id, IndexRange>,
    pub meshes: HashMap<Id, Mesh>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = id::NONE.to_id(&mut interner);
        let any = id_static("automancy", "#any").to_id(&mut interner);

        let data_ids = DataIds::new(&mut interner);
        let model_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
        let tile_ids = TileIds::new(&mut interner);
        let err_ids = ErrorIds::new(&mut interner);

        Self {
            interner,
            track,
            error_man: Default::default(),

            registry: Registry {
                tiles: Default::default(),
                scripts: Default::default(),
                tags: Default::default(),
                items: Default::default(),

                none,
                any,

                data_ids,
                model_ids,
                tile_ids,
                gui_ids,
                err_ids,
            },

            translates: Default::default(),
            audio: Default::default(),
            shaders: Default::default(),

            ordered_tiles: vec![],
            ordered_items: vec![],
            index_ranges: Default::default(),
            meshes: Default::default(),
        }
    }
}
