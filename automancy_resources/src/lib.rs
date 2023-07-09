use std::cmp::min;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Neg, Sub};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

pub use chrono;
use chrono::{DateTime, Local};
pub use kira;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use rhai::{Dynamic, Engine, Module, Scope, AST};
use walkdir::WalkDir;

use automancy_defs::coord::TileCoord;
use automancy_defs::flexstr::SharedStr;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_defs::rendering::Mesh;

use crate::data::inventory::Inventory;
use crate::data::item::{rhai_item_match, rhai_item_matches, rhai_item_stack_matches, Item};
use crate::data::stack::{ItemAmount, ItemStack};
use crate::data::{Data, DataMap};
use crate::error::ErrorManager;
use crate::model::IndexRange;
use crate::registry::{DataIds, ErrorIds, GuiIds, ModelIds, Registry};
use crate::script::{Instructions, Script};
use crate::tag::Tag;
use crate::tile::Tile;
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
    pub functions: HashMap<Id, (AST, Scope<'static>)>,

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

        engine.register_fn("item_match", rhai_item_match);
        engine.register_fn("item_matches", rhai_item_matches);
        engine.register_fn("item_matches", rhai_item_stack_matches);

        {
            let mut module = Module::new();

            module
                .set_var("TOP_RIGHT", TileCoord::TOP_RIGHT)
                .set_var("RIGHT", TileCoord::RIGHT)
                .set_var("BOTTOM_RIGHT", TileCoord::BOTTOM_RIGHT)
                .set_var("BOTTOM_LEFT", TileCoord::BOTTOM_LEFT)
                .set_var("LEFT", TileCoord::LEFT)
                .set_var("TOP_LEFT", TileCoord::TOP_LEFT);

            engine.register_static_module("TileCoord", module.into());

            engine
                .register_fn("+", TileCoord::add)
                .register_fn("-", TileCoord::sub)
                .register_fn("-", TileCoord::neg)
                .register_fn("==", |a: TileCoord, b: TileCoord| a == b)
                .register_fn("!=", |a: TileCoord, b: TileCoord| a != b);
        }

        {
            engine
                .register_indexer_get_set(DataMap::rhai_get, DataMap::rhai_set)
                .register_fn("get_or_insert", DataMap::rhai_get_or_insert);

            engine
                .register_fn("inventory", Data::rhai_inventory)
                .register_fn("amount", Data::rhai_amount)
                .register_fn("bool", Data::rhai_bool)
                .register_fn("id", Data::rhai_id)
                .register_fn("vec_id", Data::rhai_vec_id)
                .register_fn("coord", Data::rhai_coord)
                .register_fn("vec_coord", Data::rhai_vec_coord);

            engine
                .register_type_with_name::<TileCoord>("TileCoord")
                .register_fn("to_string", |coord: &mut TileCoord| coord.to_string())
                .register_iterator::<Vec<TileCoord>>();
            engine
                .register_type_with_name::<Inventory>("Inventory")
                .register_fn(
                    "take",
                    |v: &mut Inventory, item: Item, amount: ItemAmount| v.take(item, amount),
                )
                .register_indexer_get_set(Inventory::get, Inventory::insert);
            engine
                .register_type_with_name::<Id>("Id")
                .register_iterator::<Vec<Id>>();
            engine
                .register_type_with_name::<Script>("Script")
                .register_get("instructions", |v: &mut Script| v.instructions.clone());
            engine
                .register_type_with_name::<Instructions>("Instructions")
                .register_get("inputs", |v: &mut Instructions| match &v.inputs {
                    Some(v) => Dynamic::from(v.clone()),
                    None => Dynamic::UNIT,
                })
                .register_get("outputs", |v: &mut Instructions| v.outputs.clone());
            engine.register_type_with_name::<Tile>("Tile");
            engine
                .register_type_with_name::<Item>("Item")
                .register_iterator::<Vec<Item>>()
                .register_get("id", |v: &mut Item| v.id)
                .register_fn("==", |a: Item, b: Item| a == b)
                .register_fn("!=", |a: Item, b: Item| a != b);

            engine
                .register_type_with_name::<ItemStack>("ItemStack")
                .register_iterator::<Vec<ItemStack>>()
                .register_fn("ItemStack", |item: Item, amount: ItemAmount| ItemStack {
                    item,
                    amount,
                })
                .register_get("item", |v: &mut ItemStack| v.item)
                .register_get("amount", |v: &mut ItemStack| v.amount);
            engine.register_type_with_name::<Tag>("Tag");
        }

        {
            engine.register_fn("as_script", |id: Id| {
                match RESOURCE_MAN
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .registry
                    .script(id)
                    .cloned()
                {
                    Some(v) => Dynamic::from(v),
                    None => Dynamic::UNIT,
                }
            });
            engine.register_fn("as_tile", |id: Id| {
                match RESOURCE_MAN
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .registry
                    .tile(id)
                    .cloned()
                {
                    Some(v) => Dynamic::from(v),
                    None => Dynamic::UNIT,
                }
            });
            engine.register_fn("as_item", |id: Id| {
                match RESOURCE_MAN
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .registry
                    .item(id)
                    .cloned()
                {
                    Some(v) => Dynamic::from(v),
                    None => Dynamic::UNIT,
                }
            });
            engine.register_fn("as_tag", |id: Id| {
                match RESOURCE_MAN
                    .read()
                    .unwrap()
                    .clone()
                    .unwrap()
                    .registry
                    .tag(id)
                    .cloned()
                {
                    Some(v) => Dynamic::from(v),
                    None => Dynamic::UNIT,
                }
            });
        }

        {
            engine.register_fn("min", |a: i32, b: i32| min(a, b)); //TODO rhai bs
        }

        let data_ids = DataIds::new(&mut interner);
        let model_ids = ModelIds::new(&mut interner);
        let gui_ids = GuiIds::new(&mut interner);
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
