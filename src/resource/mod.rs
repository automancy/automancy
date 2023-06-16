use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};

use flexstr::SharedStr;
use hashbrown::HashMap;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use serde::Deserialize;
use walkdir::WalkDir;

use crate::game::run::error::{ErrorIds, ErrorManager};
use crate::render::data::{Face, GameVertex, Model};
use crate::render::gui::GuiIds;
use crate::resource::item::Item;
use crate::resource::model::Mesh;
use crate::resource::script::Script;
use crate::resource::tag::Tag;
use crate::resource::tile::{DataIds, Tile, TileIds};
use crate::resource::translate::Translate;
use crate::util::id::{id_static, Id, IdRaw, Interner};

pub mod audio;
//pub mod function;
pub mod item;
pub mod model;
pub mod script;
pub mod tag;
pub mod tile;
pub mod translate;

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

/// Represents the resource registry.
#[derive(Clone)]
pub struct Registry {
    pub(crate) tiles: HashMap<Id, Tile>,
    pub(crate) scripts: HashMap<Id, Script>,
    pub(crate) tags: HashMap<Id, Tag>,
    pub(crate) items: HashMap<Id, Item>,

    pub none: Id,
    pub any: Id,

    pub data_ids: DataIds,
    pub tile_ids: TileIds,
    pub gui_ids: GuiIds,
    pub err_ids: ErrorIds,
}

impl Registry {
    pub fn tile(&self, id: Id) -> Option<&Tile> {
        self.tiles.get(&id)
    }

    pub fn script(&self, id: Id) -> Option<&Script> {
        self.scripts.get(&id)
    }

    pub fn tag(&self, id: Id) -> Option<&Tag> {
        self.tags.get(&id)
    }

    pub fn item(&self, id: Id) -> Option<&Item> {
        self.items.get(&id)
    }
}
/// Represents a resource manager, which contains all resources (apart from maps) loaded from disk dynamically.
pub struct ResourceManager {
    pub interner: Interner,
    pub track: TrackHandle,

    pub error_man: ErrorManager,
    pub ordered_tiles: Vec<Id>,
    pub ordered_items: Vec<Id>,

    pub registry: Registry,

    pub translates: Translate,
    //pub functions: HashMap<Id, Function>,
    pub audio: HashMap<SharedStr, StaticSoundData>,
    pub meshes: HashMap<Id, Mesh>,

    pub all_vertices: Vec<GameVertex>,
    pub raw_models: HashMap<Id, Model>,
    pub faces: Vec<Face>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = IdRaw::NONE.to_id(&mut interner);
        let any = id_static("automancy", "#any").to_id(&mut interner);
        let data_ids = DataIds::new(&mut interner);
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
                tile_ids,
                gui_ids,
                err_ids,
            },

            translates: Default::default(),
            //functions: Default::default(),
            audio: Default::default(),
            meshes: Default::default(),

            all_vertices: Default::default(),
            raw_models: Default::default(),
            faces: Default::default(),
        }
    }
}
