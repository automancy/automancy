pub mod registry;
pub mod types;

use core::mem::ManuallyDrop;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt,
    fmt::{Debug, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

use automancy_data::{
    id,
    id::{CategoryId, GLOBAL_INTERNER, IdInterner, ItemId, ModelId, ScriptId, TileId},
    id_map::{IdMap, ImmutableIdMap},
};
use hashbrown::HashMap;
use kira::sound::static_sound::StaticSoundData;
use rhai::Engine;
use thiserror::Error;
use walkdir::WalkDir;

use crate::{
    resources::{
        registry::{MutableRegistry, Registry},
        types::{
            font::FontData,
            script::ScriptData,
            translate::{MutableTranslateDef, TranslateDef},
        },
    },
    scripting,
};

pub static RESOURCES_PATH: &str = "resources";

pub(crate) static FONT_EXTS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];
pub(crate) static RON_EXTS: [&str; 1] = ["ron"];
pub(crate) static SCRIPT_EXTS: [&str; 1] = ["rhai"];
pub(crate) static SHADER_EXTS: [&str; 1] = ["wgsl"];

/// TODO more audio formats are supported
pub(crate) static AUDIO_EXTS: [&str; 1] = ["ogg"];

pub(crate) fn read_recursively<const LEN: usize, S: Into<OsString>>(path: &Path, valid_exts: [S; LEN]) -> Vec<PathBuf> {
    let valid_exts = valid_exts.map(Into::into);

    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|entry| {
            let ext = entry.path().extension().unwrap_or_default();

            valid_exts.iter().any(|valid| ext.eq_ignore_ascii_case(valid))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("file \"{0}\" is invalid: could not get file stem")]
    NoFileStem(PathBuf),
    #[error("file \"{0}\" is invalid: could not convert OsString to String")]
    OsStringError(PathBuf),

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
            err => {
                log::error!("Error loading resources: {err}.");
            },
        }
    }
}

pub mod global {
    use std::sync::{Arc, RwLock};

    use automancy_data::id::IdLike;

    use super::ResourceManager;

    static RESOURCE_MAN: RwLock<Option<Arc<ResourceManager>>> = RwLock::new(None);

    pub fn resource_man() -> Arc<ResourceManager> {
        RESOURCE_MAN.read().unwrap().as_ref().unwrap().clone()
    }

    pub fn set_resource_man(resource_man: Arc<ResourceManager>) {
        RESOURCE_MAN.write().unwrap().replace(resource_man);
    }

    pub fn debug_id<Id: IdLike>(id: Id) -> String {
        resource_man().interner.resolve(id.into()).unwrap_or("invalid").to_string()
    }
}

/// Represents a "Resource Manager", which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: IdInterner,
    pub registry: Registry,

    pub engine: Engine,
    pub scripts: ImmutableIdMap<ScriptId, ScriptData>,

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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

/// Mutable version of [`ResourceManager`].
pub struct MutableResourceManager {
    /// SAFETY: the `MutableResourceManager` should never be shared between threads.
    pub(crate) interner: ManuallyDrop<&'static mut IdInterner>,
    pub(crate) registry: MutableRegistry,

    pub(crate) engine: Engine,
    pub(crate) scripts: IdMap<ScriptId, ScriptData>,

    pub(crate) translates: MutableTranslateDef,
    pub(crate) models: IdMap<ModelId, (gltf::Document, Vec<gltf::buffer::Data>)>,
    pub(crate) audio: HashMap<String, StaticSoundData>,
    pub(crate) shaders: HashMap<String, String>,
    // uses a BTreeMap so font iterations stay sorted and ordered
    pub(crate) fonts: BTreeMap<String, Vec<FontData>>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut interner = ManuallyDrop::new(unsafe { &mut *GLOBAL_INTERNER.with(|interner| interner.get()) });
        let registry = MutableRegistry::new(&mut interner);

        let mut engine = Engine::new();
        engine.set_max_expr_depths(0, 0);
        engine.set_fast_operators(true);

        scripting::coord::register_coord_stuff(&mut engine);
        scripting::data::register_data_stuff(&mut engine);
        scripting::math::register_math_stuff(&mut engine);
        scripting::render::register_render_stuff(&mut engine);
        scripting::tile::register_tile_stuff(&mut engine);
        scripting::ui::register_ui_stuff(&mut engine);
        scripting::util::register_script_stuff(&mut engine);

        Self {
            interner,
            registry,

            engine,
            scripts: Default::default(),

            translates: Default::default(),
            audio: Default::default(),
            shaders: Default::default(),
            fonts: Default::default(),
            models: Default::default(),
        }
    }

    pub fn compile(mut resource_man: Self) -> Arc<ResourceManager> {
        resource_man
            .engine
            .definitions()
            .with_headers(true)
            .include_standard_packages(false)
            .write_to_dir("rhai")
            .unwrap();

        let (ordered_categories, category_tiles_map) = resource_man.compile_categories();
        let research_unlock_map = resource_man.compile_researches();
        let ordered_items = resource_man.compile_ordered_items();
        let ordered_tiles = resource_man.compile_ordered_tiles();

        let resource_man = Arc::new(ResourceManager {
            interner: Clone::clone(&resource_man.interner),
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

            engine: resource_man.engine,
            scripts: resource_man.scripts.into(),

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

        resource_man
    }
}
