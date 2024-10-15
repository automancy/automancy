use crate::registry::{DataIds, ErrorIds, GuiIds, KeyIds, ModelIds, Registry};
use crate::types::font::Font;
use crate::types::model::IndexRange;
use crate::types::translate::TranslateDef;
use automancy_defs::rendering::{Animation, Mesh};
use automancy_defs::{
    chrono::{DateTime, Local},
    id::SharedStr,
};
use automancy_defs::{coord::TileCoord, log};
use automancy_defs::{id::ModelId, kira::sound::static_sound::StaticSoundData};
use automancy_defs::{id::TileId, kira::track::TrackHandle};
use automancy_defs::{
    id::{Id, IdRaw, Interner},
    stack::ItemStack,
};
use hashbrown::HashMap;
use rhai::{CallFnOptions, Dynamic, Engine, AST};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use thiserror::Error;
use types::item::ItemDef;
use walkdir::WalkDir;

pub use petgraph;

pub mod data;
pub mod error;
pub mod inventory;

pub mod format;
pub mod registry;
pub mod types;

pub mod rhai_coord;
pub mod rhai_data;
pub mod rhai_math;
pub mod rhai_render;
pub mod rhai_resources;
pub mod rhai_tile;
pub mod rhai_ui;
pub mod rhai_utils;

pub type FunctionInfo = (AST, String);

pub static RESOURCES_PATH: &str = "resources";

pub static FONT_EXT: [&str; 2] = ["ttf", "otf"];
pub static RON_EXT: &str = "ron";
pub static FUNCTION_EXT: &str = "rhai";
pub static SHADER_EXT: &str = "wgsl";

/// TODO set of extensions
pub static AUDIO_EXT: &str = "ogg";

static COULD_NOT_GET_FILE_STEM: &str = "could not get file stem";

/// Converts a UTC Unix timestamp into a formatted time string, using the given strftime format string.
pub fn format_time(time: SystemTime, fmt: &str) -> String {
    let time = DateTime::<Local>::from(time);
    time.format(fmt).to_string()
}

pub(crate) fn load_recursively(path: &Path, extension: &OsStr) -> Vec<PathBuf> {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|v| v.path().extension() == Some(extension))
        .map(|v| v.path().to_path_buf())
        .collect()
}

#[derive(Error, Debug)]
pub enum LoadResourceError {
    #[error("the file {0} is invalid: {1}")]
    InvalidFileError(PathBuf, &'static str),
    #[error("could not convert OsString to String of path {0}")]
    OsStringError(PathBuf),
    #[error("could not get font name from {0}")]
    CouldNotGetFontName(PathBuf),
}

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("item could not be found")]
    ItemNotFound,
}

pub static RESOURCE_MAN: RwLock<Option<Arc<ResourceManager>>> = RwLock::new(None);

/// Represents a resource manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,
    pub engine: Engine,

    pub registry: Registry,

    pub translates: TranslateDef,
    pub audio: HashMap<String, StaticSoundData>,
    pub shaders: HashMap<String, SharedStr>,
    pub functions: HashMap<Id, FunctionInfo>,
    pub fonts: BTreeMap<String, Font>, // yes this does need to be a BTreeMap

    pub ordered_tiles: Vec<TileId>,
    pub ordered_items: Vec<Id>,
    pub ordered_categories: Vec<Id>,
    pub all_models: HashMap<ModelId, (Vec<Option<Mesh>>, Vec<Animation>)>,
    pub all_index_ranges: HashMap<ModelId, HashMap<usize, IndexRange>>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = IdRaw::new("core", "none").to_id(&mut interner);
        let any = IdRaw::new("core", "#any").to_id(&mut interner);

        let mut engine = Engine::new();
        engine.set_max_expr_depths(0, 0);
        engine.set_fast_operators(false);

        rhai_math::register_math_stuff(&mut engine);
        rhai_utils::register_functions(&mut engine);
        rhai_coord::register_coord_stuff(&mut engine);
        rhai_data::register_data_stuff(&mut engine);
        rhai_resources::register_resources(&mut engine);
        rhai_tile::register_tile_stuff(&mut engine);
        rhai_ui::register_ui_stuff(&mut engine);
        rhai_render::register_render_stuff(&mut engine);

        let data_ids = DataIds::new(&mut interner);
        let model_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
        let key_ids = KeyIds::new(&mut interner);
        let err_ids = ErrorIds::new(&mut interner);

        Self {
            interner,
            track,
            engine,

            registry: Registry {
                tiles: Default::default(),
                scripts: Default::default(),
                tags: Default::default(),
                categories: Default::default(),
                categories_tiles_map: Default::default(),
                items: Default::default(),
                researches: Default::default(),
                researches_id_map: Default::default(),
                researches_unlock_map: Default::default(),

                none,
                any,

                data_ids,
                model_ids,
                gui_ids,
                err_ids,
                key_ids,
            },

            translates: Default::default(),
            audio: Default::default(),
            shaders: Default::default(),
            functions: Default::default(),
            fonts: Default::default(),

            ordered_tiles: vec![],
            ordered_items: vec![],
            ordered_categories: vec![],
            all_index_ranges: Default::default(),
            all_models: Default::default(),
        }
    }
}

pub fn rhai_call_options(state: &mut Dynamic) -> CallFnOptions {
    CallFnOptions::new()
        .eval_ast(false)
        .rewind_scope(true)
        .bind_this_ptr(state)
}

pub fn rhai_log_err(
    called_func: &str,
    function_id: &str,
    err: &rhai::EvalAltResult,
    coord: Option<TileCoord>,
) {
    let coord = coord
        .map(|v| v.to_minimal_string())
        .unwrap_or_else(|| "(no coord available)".to_string());

    match err {
        rhai::EvalAltResult::ErrorFunctionNotFound(name, ..) => {
            if name != called_func {
                log::error!("At {coord}, In {function_id}, {called_func}: {err}");
            }
        }
        _ => {
            log::error!("At {coord}, In {function_id}, {called_func}: {err}");
        }
    }
}

pub fn item_match(resource_man: &ResourceManager, id: Id, other: Id) -> bool {
    if let Some(tag) = resource_man.registry.tags.get(&other) {
        return tag.of(&resource_man.registry, id);
    }

    if id == other {
        return true;
    }

    false
}

pub fn item_matches(
    resource_man: &ResourceManager,
    id: Id,
    mut others: impl Iterator<Item = ItemDef>,
) -> Option<ItemDef> {
    others.find(|&other| item_match(resource_man, id, other.id))
}

pub fn item_stack_matches(
    resource_man: &ResourceManager,
    id: Id,
    mut others: impl Iterator<Item = ItemStack>,
) -> Option<ItemStack> {
    others.find(|&other| item_match(resource_man, id, other.id))
}

pub fn item_ids_of_tag(resource_man: &ResourceManager, id: Id) -> Vec<Id> {
    resource_man
        .ordered_items
        .iter()
        .filter(|v| item_match(resource_man, **v, id))
        .cloned()
        .collect()
}
