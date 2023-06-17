use chrono::{DateTime, Local, Utc};
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use crate::registry::{DataIds, ErrorIds, GuiIds, ModelIds, Registry, TileIds};

use crate::error::ErrorManager;
use crate::model::Mesh;
use crate::translate::Translate;
use automancy_defs::flexstr::SharedStr;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_defs::rendering::{Face, GameVertex, Model};
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use walkdir::WalkDir;

pub extern crate chrono;
pub extern crate kira;

pub mod audio;
pub mod data;
pub mod error;
pub mod item;
pub mod model;
pub mod registry;
pub mod script;
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
pub fn unix_to_formatted_time(utc: i64, fmt: &str) -> String {
    let from_epoch = UNIX_EPOCH + Duration::from_secs(utc as u64);
    let past = DateTime::<Utc>::from(from_epoch);
    let time = DateTime::<Local>::from(past);
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

pub static JSON_EXT: &str = "json";
pub static OGG_EXT: &str = "ogg";
pub static PNG_EXT: &str = "png";
pub static RESOURCES_FOLDER: &str = "resources";

/// Represents a automancy_resources manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,

    pub error_man: ErrorManager,
    pub ordered_tiles: Vec<Id>,
    pub ordered_items: Vec<Id>,

    pub registry: Registry,

    pub translates: Translate,
    pub audio: HashMap<SharedStr, StaticSoundData>,
    pub meshes: HashMap<Id, Mesh>,

    pub all_vertices: Vec<GameVertex>,
    pub raw_models: HashMap<Id, Model>,
    pub faces: Vec<Face>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<automancy_resources manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = id::NONE.to_id(&mut interner);
        let any = id_static("automancy", "#any").to_id(&mut interner);
        let data_ids = DataIds::new(&mut interner); //TODO Registry::new
        let item_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
        let tile_ids = TileIds::new(&mut interner);
        let err_ids = ErrorIds::new(&mut interner);

        Self {
            interner,
            track,

            error_man: Default::default(),
            ordered_tiles: vec![],
            ordered_items: vec![],

            registry: Registry {
                tiles: Default::default(),
                scripts: Default::default(),
                tags: Default::default(),
                items: Default::default(),

                none,
                any,

                data_ids,
                model_ids: item_ids,
                tile_ids,
                gui_ids,
                err_ids,
            },

            translates: Default::default(),
            audio: Default::default(),
            meshes: Default::default(),

            all_vertices: Default::default(),
            raw_models: Default::default(),
            faces: Default::default(),
        }
    }
}
