use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

pub use chrono;
use chrono::{DateTime, Local};
use hashbrown::HashMap;
pub use kira;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use registry::KeyIds;
use rhai::{CallFnOptions, Dynamic, Engine, Scope, AST};
use thiserror::Error;
use walkdir::WalkDir;

use automancy_defs::flexstr::SharedStr;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_defs::rendering::{Animation, Model};
use automancy_defs::{id, log};

use crate::error::ErrorManager;
use crate::registry::{DataIds, ErrorIds, GuiIds, ModelIds, Registry};
use crate::types::font::Font;
use crate::types::model::IndexRange;
use crate::types::translate::Translate;

pub mod data;
pub mod error;

pub mod registry;

pub mod types;

mod rhai_coord;
mod rhai_data;
mod rhai_functions;
mod rhai_resources;
mod rhai_tile;

static COULD_NOT_GET_FILE_STEM: &str = "could not get file stem";

#[derive(Error, Debug)]
pub enum LoadResourceError {
    #[error("the file {0} is invalid: {1}")]
    InvalidFileError(PathBuf, &'static str),
    #[error("could not convert OsString to String")]
    OsStringError(PathBuf),
}

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

pub const FONT_EXT: [&str; 2] = ["ttf", "otf"];
pub const RON_EXT: &str = "ron";
pub const AUDIO_EXT: &str = "ogg";
pub const FUNCTION_EXT: &str = "rhai";
pub const SHADER_EXT: &str = "wgsl";
pub const IMAGE_EXT: &str = "png";

/// TODO set of extensions

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("item could not be found")]
    ItemNotFound,
}

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
    pub functions: HashMap<Id, (AST, Scope<'static>, String)>,
    pub fonts: BTreeMap<String, Font>, // yes this does need to be a BTreeMap

    pub ordered_tiles: Vec<Id>,
    pub ordered_items: Vec<Id>,
    pub ordered_categories: Vec<Id>,
    pub all_models: HashMap<Id, (HashMap<usize, Model>, Vec<Animation>)>,
    pub all_index_ranges: HashMap<Id, HashMap<usize, IndexRange>>,
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
        engine.set_max_expr_depths(0, 0);
        engine.set_fast_operators(false);

        rhai_functions::register_functions(&mut engine);
        rhai_coord::register_coord_stuff(&mut engine);
        rhai_data::register_data_stuff(&mut engine);
        rhai_resources::register_resources(&mut engine);
        rhai_tile::register_tile_stuff(&mut engine);

        let data_ids = DataIds::new(&mut interner);
        let model_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
        let key_ids = KeyIds::new(&mut interner);
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
                categories: Default::default(),
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

    pub fn item_name(&self, id: &Id) -> SharedStr {
        match self.translates.items.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn try_item_name(&self, id: Option<&Id>) -> SharedStr {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            self.translates.none.clone()
        }
    }

    pub fn script_name(&self, id: &Id) -> SharedStr {
        match self.translates.scripts.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn try_script_name(&self, id: Option<&Id>) -> SharedStr {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            self.translates.none.clone()
        }
    }

    pub fn tile_name(&self, id: &Id) -> SharedStr {
        match self.translates.tiles.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn try_tile_name(&self, id: Option<&Id>) -> SharedStr {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            self.translates.none.clone()
        }
    }

    pub fn category_name(&self, id: &Id) -> SharedStr {
        match self.translates.categories.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn try_category_name(&self, id: Option<&Id>) -> SharedStr {
        if let Some(id) = id {
            self.category_name(id)
        } else {
            self.translates.none.clone()
        }
    }

    pub fn gui_str(&self, id: &Id) -> SharedStr {
        match self.translates.gui.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn research_str(&self, id: &Id) -> SharedStr {
        match self.translates.research.get(id) {
            Some(name) => name.clone(),
            None => self.translates.unnamed.clone(),
        }
    }

    pub fn try_research_str(&self, id: Option<&Id>) -> SharedStr {
        if let Some(id) = id {
            self.research_str(id)
        } else {
            self.translates.none.clone()
        }
    }
}

pub fn rhai_call_options(state: &mut Dynamic) -> CallFnOptions {
    CallFnOptions::new()
        .eval_ast(false)
        .rewind_scope(true)
        .bind_this_ptr(state)
}

pub fn rhai_log_err(function_id: &str, err: &rhai::EvalAltResult) {
    match err {
        rhai::EvalAltResult::ErrorFunctionNotFound(..) => {}
        _ => {
            log::error!("In {function_id}: {err}");
        }
    }
}
