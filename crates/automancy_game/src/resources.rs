pub mod registry;
pub mod types;

use core::cell::{Cell, RefCell};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

use automancy_data::{
    self, id,
    id::{CategoryId, GLOBAL_INTERNER, IdInterner, ItemId, MUTABLE_GLOBAL_INTERNER, ModelId, ScriptId, TileId},
    id_map::{IdMap, ImmutableIdMap},
};
use hashbrown::HashMap;
use kira::sound::static_sound::StaticSoundData;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::resources::{
    registry::{MutableRegistry, Registry},
    types::{
        font::FontData,
        script::RhaiScriptData,
        translate::{MutableTranslateDef, TranslateDef},
    },
};

pub static RESOURCES_PATH: &str = "resources";
pub static RESOURCES_CORE_PATH: &str = "core";
pub static RESOURCES_AUTOMANCY_PATH: &str = "automancy";

pub(crate) static SCRIPTS_PATH: &str = "scripts";
pub(crate) static RHAI_EXT: &str = "rhai";

pub(crate) static FONT_EXTS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];
pub(crate) static RON_EXTS: [&str; 1] = ["ron"];
pub(crate) static SCRIPT_EXTS: [&str; 1] = ["rhai"];
pub(crate) static SHADER_EXTS: [&str; 1] = ["wgsl"];

/// TODO more audio formats are supported
pub(crate) static AUDIO_EXTS: [&str; 1] = ["ogg"];

pub(crate) fn read_recursively<const LEN: usize>(path: &Path, valid_exts: [&'static str; LEN]) -> impl Iterator<Item = DirEntry> {
    let valid_exts = valid_exts.map(OsStr::new);

    WalkDir::new(path).follow_links(false).into_iter().flatten().filter(move |entry| {
        let ext = entry.path().extension();

        ext.is_some_and(|ext| valid_exts.iter().any(|valid| ext.eq_ignore_ascii_case(valid)))
    })
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("file \"{0}\" is invalid: could not get file stem")]
    NoFileStem(PathBuf),
    #[error("file \"{0}\" is invalid: could not convert OsString to String")]
    OsStringError(PathBuf),
    #[error("file \"{0}\" is invalid: path cannot contain '.' (dot), please rename your files")]
    PathContainsDot(PathBuf),

    #[error("font file \"{0}\" is invalid: could not parse font: {1}")]
    CouldNotParseFont(PathBuf, ttf_parser::FaceParsingError),
    #[error("font file \"{0}\" is invalid: could not get font name")]
    CouldNotGetFontName(PathBuf),

    #[error("{ty} file \"{file}\" is invalid: built-in {ty} of name '{name}' should not be redefined")]
    BuiltInRedefined { ty: &'static str, file: PathBuf, name: String },

    #[error(transparent)]
    StrIdParseError(#[from] id::deserialize::StrIdParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    RonSpannedError(#[from] ron::de::SpannedError),
    #[error(transparent)]
    GltfError(#[from] gltf::Error),
    #[error(transparent)]
    RhaiEvalError(#[from] Box<rhai::EvalAltResult>),
    #[error(transparent)]
    RhaiParseError(#[from] rhai::ParseError),
    #[error(transparent)]
    KiraParseError(#[from] kira::sound::FromFileError),
}

impl ResourceError {
    pub fn log_err(&self) {
        match self {
            err @ ResourceError::BuiltInRedefined {
                ..
            } => {
                log::error!("Error loading resources: {err}.");
                log::error!("Ignoring redefined built-in item.");
            },
            err @ ResourceError::RhaiParseError(..) => {
                log::error!("Error parsing rhai script: {err}.");
            },
            err @ ResourceError::RhaiEvalError(..) => {
                log::error!("Error evaluating rhai: {err}.");
            },
            err @ ResourceError::KiraParseError(..) => {
                log::error!("Error loading audio file: {err}.");
            },
            err => {
                log::error!("Error loading resources: {err}.");
            },
        }
    }
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

/// Represents a "Resource Manager", which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Arc<IdInterner>,
    pub registry: Registry,

    pub rhai: rhai::Engine,
    pub rhai_scripts: ImmutableIdMap<ScriptId, RhaiScriptData>,

    pub translates: TranslateDef,
    pub models: ImmutableIdMap<ModelId, (gltf::Document, Vec<gltf::buffer::Data>)>,
    pub audio: HashMap<String, StaticSoundData>,
    pub shaders: HashMap<String, String>,
    pub fonts: BTreeMap<String, Vec<FontData>>,

    pub ordered_tiles: Vec<TileId>,
    pub ordered_items: Vec<ItemId>,
    pub ordered_categories: Vec<CategoryId>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<resource manager>")
    }
}

thread_local! {
    static MUTABLE_RESOURCE_MAN: RefCell<Option<MutableResourceManager>> = const { RefCell::new(None) };
    pub(crate) static UNIT_TESTS_ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// Mutable version of [`ResourceManager`].
pub struct MutableResourceManager {
    pub(crate) registry: MutableRegistry,

    pub(crate) rhai_scripts: IdMap<ScriptId, RhaiScriptData>,

    pub(crate) translates: MutableTranslateDef,
    pub(crate) models: IdMap<ModelId, (gltf::Document, Vec<gltf::buffer::Data>)>,
    pub(crate) audio: HashMap<String, StaticSoundData>,
    pub(crate) shaders: HashMap<String, String>,
    // uses a BTreeMap so font iterations stay sorted and ordered
    pub(crate) fonts: BTreeMap<String, Vec<FontData>>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    pub fn enable_unit_tests() {
        log::info!("Unit tests are enabled in this environment! The game will crash if tests fail.");
        UNIT_TESTS_ENABLED.set(true);
    }

    pub fn setup(rhai: &mut rhai::Engine) {
        let mut interner = IdInterner::new();
        let registry = MutableRegistry::new(&mut interner);
        // make sure to immediately set the interner after initialization
        MUTABLE_GLOBAL_INTERNER.set(Some(interner));

        {
            rhai.set_max_expr_depths(0, 0);
            rhai.set_fast_operators(true);

            use crate::scripting_rhai;

            scripting_rhai::coord::register_coord_stuff(rhai);
            scripting_rhai::data::register_data_stuff(rhai);
            scripting_rhai::math::register_math_stuff(rhai);
            scripting_rhai::render::register_render_stuff(rhai);
            scripting_rhai::tile::register_tile_stuff(rhai);
            scripting_rhai::ui::register_ui_stuff(rhai);
            scripting_rhai::util::register_script_stuff(rhai);
        }

        MUTABLE_RESOURCE_MAN.set(Some(Self {
            registry,

            rhai_scripts: Default::default(),

            translates: Default::default(),
            audio: Default::default(),
            shaders: Default::default(),
            fonts: Default::default(),
            models: Default::default(),
        }));
    }

    #[inline]
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MutableResourceManager) -> R,
    {
        MUTABLE_RESOURCE_MAN.with_borrow_mut(|resource_man| {
            let resource_man = resource_man
                .as_mut()
                .expect("MutableResourceManager::with must be called after MutableResourceManager::setup has been called.");

            f(resource_man)
        })
    }

    #[inline]
    pub fn with_interner<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MutableResourceManager, &mut IdInterner) -> R,
    {
        MUTABLE_RESOURCE_MAN.with_borrow_mut(|resource_man| {
            MUTABLE_GLOBAL_INTERNER.with_borrow_mut(|interner| {
                let (resource_man, interner) = resource_man
                    .as_mut()
                    .zip(interner.as_mut())
                    .expect("MutableResourceManager::with_interner must be called after MutableResourceManager::setup has been called.");

                f(resource_man, interner)
            })
        })
    }

    pub fn compile(rhai: rhai::Engine) -> Arc<ResourceManager> {
        {
            rhai.definitions()
                .with_headers(true)
                .include_standard_packages(false)
                .write_to_dir("rhai")
                .unwrap();
        }

        let mut resource_man = MUTABLE_RESOURCE_MAN.take().unwrap();
        let (ordered_categories, category_tiles_map) = resource_man.compile_categories();
        let research_unlock_map = resource_man.compile_researches();
        let ordered_items = resource_man.compile_ordered_items();
        let ordered_tiles = resource_man.compile_ordered_tiles();

        let resource_man = Arc::new(ResourceManager {
            // make sure to only move the interner out as we initialize the resource manager
            interner: Arc::new(Clone::clone(&MUTABLE_GLOBAL_INTERNER.take().unwrap())),
            registry: Registry {
                tile_defs: resource_man.registry.tile_defs.into(),
                item_defs: resource_man.registry.item_defs.into(),
                recipe_defs: resource_man.registry.recipe_defs.into(),
                tag_defs: resource_man.registry.tag_defs.into(),
                category_defs: resource_man.registry.category_defs.into(),
                research_defs: resource_man.registry.research_defs,
                research_id_map: resource_man.registry.research_id_map.into(),

                data_ids: resource_man.registry.data_ids,
                model_ids: resource_man.registry.model_ids,
                gui_ids: resource_man.registry.gui_ids,
                key_ids: resource_man.registry.key_ids,
                err_ids: resource_man.registry.err_ids,
                render_ids: resource_man.registry.render_ids,

                category_tiles_map: category_tiles_map.into(),
                research_unlock_map: research_unlock_map.into(),
            },

            rhai,
            rhai_scripts: resource_man.rhai_scripts.into(),

            translates: TranslateDef {
                none: resource_man.translates.none,
                unnamed: resource_man.translates.unnamed,

                items: resource_man.translates.items.into(),
                tiles: resource_man.translates.tiles.into(),
                categories: resource_man.translates.categories.into(),
                recipes: resource_man.translates.recipes.into(),
                researches: resource_man.translates.researches.into(),

                gui: resource_man.translates.gui.into(),
                error: resource_man.translates.error.into(),
                keys: resource_man.translates.keys.into(),
            },
            models: resource_man.models.into(),
            audio: resource_man.audio,
            shaders: resource_man.shaders,
            fonts: resource_man.fonts,

            ordered_tiles,
            ordered_items,
            ordered_categories,
        });

        global::set_resource_man(resource_man.clone());
        *GLOBAL_INTERNER.write().unwrap() = resource_man.interner.clone();

        resource_man
    }
}
