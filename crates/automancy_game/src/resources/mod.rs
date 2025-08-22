pub mod error;
pub mod format;
pub mod registry;
pub mod types;

use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt,
    fmt::{Debug, Formatter},
    path::{Path, PathBuf},
    time::SystemTime,
};

use automancy_data::{
    game::coord::TileCoord,
    id::{Id, Interner, ModelId, TileId, deserialize::IdStr},
};
use chrono::{DateTime, Local};
use hashbrown::HashMap;
use kira::{sound::static_sound::StaticSoundData, track::TrackHandle};
use rhai::{AST, CallFnOptions, Dynamic, Engine};
use thiserror::Error;
use types::function::FunctionMetadata;
use walkdir::WalkDir;

use crate::{
    resources::{
        registry::Registry,
        types::{font::Font, translate::TranslateDef},
    },
    scripting,
};

pub type FunctionInfo = (AST, FunctionMetadata);

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

pub mod global {
    use std::sync::{Arc, RwLock};

    use super::ResourceManager;

    static RESOURCE_MAN: RwLock<Option<Arc<ResourceManager>>> = RwLock::new(None);
    pub fn resource_man() -> Arc<ResourceManager> {
        RESOURCE_MAN.read().unwrap().as_ref().unwrap().clone()
    }
    pub fn set_resource_man(resource_man: Arc<ResourceManager>) {
        RESOURCE_MAN.write().unwrap().replace(resource_man);
    }
}

/// Represents a resource manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,
    pub engine: Engine,

    pub registry: Registry,

    pub translates: TranslateDef,
    pub audio: HashMap<String, StaticSoundData>,
    pub shaders: HashMap<String, String>,
    pub functions: HashMap<Id, FunctionInfo>,
    pub fonts: BTreeMap<String, Font>, // yes this does need to be a BTreeMap

    pub ordered_tiles: Vec<TileId>,
    pub ordered_items: Vec<Id>,
    pub ordered_categories: Vec<Id>,
    pub all_gltf_models: HashMap<ModelId, (gltf::Document, Vec<gltf::buffer::Data>)>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = IdStr::new("core", "none").into_id(&mut interner);
        let any = IdStr::new("core", "#any").into_id(&mut interner);

        let mut engine = Engine::new();
        engine.set_max_expr_depths(0, 0);
        engine.set_fast_operators(false);

        scripting::coord::register_coord_stuff(&mut engine);
        scripting::data::register_data_stuff(&mut engine);
        scripting::math::register_math_stuff(&mut engine);
        scripting::render::register_render_stuff(&mut engine);
        scripting::tile::register_tile_stuff(&mut engine);
        scripting::ui::register_ui_stuff(&mut engine);
        scripting::util::register_functions(&mut engine);

        let data_ids = registry::DataIds::new(&mut interner);
        let model_ids = registry::ModelIds::new(&mut interner);
        let gui_ids = registry::GuiIds::new(&mut interner);
        let key_ids = registry::KeyIds::new(&mut interner);
        let err_ids = registry::ErrorIds::new(&mut interner);

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
            all_gltf_models: Default::default(),
        }
    }
}

pub fn rhai_call_options<'a>(state: &'a mut Dynamic) -> CallFnOptions<'a> {
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
