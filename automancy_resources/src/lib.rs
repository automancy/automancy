use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

pub use chrono;
use chrono::{DateTime, Local};
pub use kira;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use rhai::{Engine, Module, AST};
use walkdir::WalkDir;

use automancy_defs::coord::TileCoord;
use automancy_defs::flexstr::SharedStr;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_defs::rendering::Mesh;

use crate::data::inventory::Inventory;
use crate::data::item::{item_match, item_match_str};
use crate::data::{Data, DataMap};
use crate::error::ErrorManager;
use crate::model::IndexRange;
use crate::registry::{DataIds, ErrorIds, GuiIds, ModelIds, Registry, TileIds};
use crate::translate::Translate;

pub mod audio;
pub mod data;
pub mod error;
pub mod function;
pub mod item;
pub mod model;
pub mod registry;
pub mod script;
pub mod shader;
pub mod tag;
pub mod tile;
pub mod translate;

pub static RESOURCE_MAN: RwLock<Option<Arc<ResourceManager>>> = RwLock::new(None);

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

pub const RESOURCES_PATH: &str = "resources";

pub const JSON_EXT: &str = "json";
pub const AUDIO_EXT: &str = "ogg";
pub const FUNCTION_EXT: &str = "rhai";
pub const SHADER_EXT: &str = "wgsl";
pub const IMAGE_EXT: &str = "png";

/// TODO set of extensions

/// Represents a resource manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,
    pub error_man: ErrorManager,
    pub engine: Engine,

    pub registry: Registry,

    pub translates: Translate,
    pub audio: HashMap<SharedStr, StaticSoundData>,
    pub shaders: HashMap<SharedStr, String>,
    pub functions: HashMap<Id, AST>,

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

        let mut engine = Engine::new();
        engine.set_fast_operators(false);

        engine.register_fn("item_match", item_match);
        engine.register_fn("item_match", item_match_str);

        {
            let mut module = Module::new();

            module.set_var("TOP_RIGHT", TileCoord::TOP_RIGHT);
            module.set_var("RIGHT", TileCoord::RIGHT);
            module.set_var("BOTTOM_RIGHT", TileCoord::BOTTOM_RIGHT);
            module.set_var("BOTTOM_LEFT", TileCoord::BOTTOM_LEFT);
            module.set_var("LEFT", TileCoord::LEFT);
            module.set_var("TOP_LEFT", TileCoord::TOP_LEFT);

            engine.register_static_module("TileCoord", module.into());

            engine.register_fn("+", |a: TileCoord, b: TileCoord| a + b);
            engine.register_fn("-", |a: TileCoord, b: TileCoord| a - b);
            engine.register_fn("==", |a: TileCoord, b: TileCoord| a == b);
            engine.register_fn("!=", |a: TileCoord, b: TileCoord| a != b);
        }

        {
            engine.register_indexer_get_set(DataMap::rhai_get, DataMap::rhai_set);

            engine.register_fn("inventory", Data::rhai_inventory);
            engine.register_fn("amount", Data::rhai_amount);
            engine.register_fn("bool", Data::rhai_bool);
            engine.register_fn("id", Data::rhai_id);
            engine.register_fn("vec_id", Data::rhai_vec_id);
            engine.register_fn("coord", Data::rhai_coord);
            engine.register_fn("vec_coord", Data::rhai_vec_coord);
            engine.register_type_with_name::<TileCoord>("TileCoord");
            engine.register_type_with_name::<Inventory>("Inventory");
            engine.register_type_with_name::<Id>("Id");
        }

        let data_ids = DataIds::new(&mut interner);
        let model_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
        let tile_ids = TileIds::new(&mut interner);
        let err_ids = ErrorIds::new(&mut interner);

        Self {
            interner,
            track,
            error_man: Default::default(),
            engine,

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
            functions: Default::default(),

            ordered_tiles: vec![],
            ordered_items: vec![],
            index_ranges: Default::default(),
            meshes: Default::default(),
        }
    }
}
