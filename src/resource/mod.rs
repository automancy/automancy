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
use crate::resource::item::Item;
use crate::resource::model::Mesh;
use crate::resource::script::Script;
use crate::resource::tag::Tag;
use crate::resource::tile::Tile;
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

#[derive(Copy, Clone)]
pub struct TileIds {
    pub machine: Id,
    pub transfer: Id,
    pub void: Id,
    pub storage: Id,
    pub merger: Id,
    pub splitter: Id,
    pub master_node: Id,
    pub node: Id,
}

impl TileIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            machine: id_static("automancy", "machine").to_id(interner),
            transfer: id_static("automancy", "transfer").to_id(interner),
            void: id_static("automancy", "void").to_id(interner),
            storage: id_static("automancy", "storage").to_id(interner),
            merger: id_static("automancy", "merger").to_id(interner),
            splitter: id_static("automancy", "splitter").to_id(interner),
            master_node: id_static("automancy", "master_node").to_id(interner),
            node: id_static("automancy", "node").to_id(interner),
        }
    }
}

#[derive(Copy, Clone)]
pub struct DataIds {
    pub script: Id,
    pub scripts: Id,
    pub buffer: Id,
    pub storage: Id,
    pub storage_type: Id,
    pub amount: Id,
    pub target: Id,
    pub link: Id,
}

impl DataIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            script: id_static("automancy", "script").to_id(interner),
            scripts: id_static("automancy", "scripts").to_id(interner),
            buffer: id_static("automancy", "buffer").to_id(interner),
            storage: id_static("automancy", "storage").to_id(interner),
            storage_type: id_static("automancy", "storage_type").to_id(interner),
            amount: id_static("automancy", "amount").to_id(interner),
            target: id_static("automancy", "target").to_id(interner),
            link: id_static("automancy", "link").to_id(interner),
        }
    }
}

#[derive(Copy, Clone)]
pub struct ModelIds {
    pub items_missing: Id,
}

impl ModelIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            items_missing: id_static("automancy", "items/missing").to_id(interner),
        }
    }
}

/// The list of GUI translation keys.
#[derive(Clone, Copy)]
pub struct GuiIds {
    pub tile_config: Id,
    pub tile_info: Id,
    pub tile_config_script: Id,
    pub tile_config_storage: Id,
    pub tile_config_target: Id,
    pub error_popup: Id,
    pub debug_menu: Id,
    pub load_map: Id,
    pub delete_map: Id,
    pub create_map: Id,
    pub options: Id,

    pub lbl_amount: Id,
    pub lbl_link_destination: Id,
    pub lbl_maps_loaded: Id,
    pub lbl_delete_map_confirm: Id,

    pub btn_confirm: Id,
    pub btn_exit: Id,
    pub btn_cancel: Id,
    pub btn_link_network: Id,
    pub btn_play: Id,
    pub btn_options: Id,
    pub btn_fedi: Id,
    pub btn_source: Id,
    pub btn_unpause: Id,
    pub btn_load: Id,
    pub btn_delete: Id,
    pub btn_new_map: Id,

    pub time_fmt: Id,
}

impl GuiIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            tile_config: id_static("automancy", "tile_config").to_id(interner),
            tile_info: id_static("automancy", "tile_info").to_id(interner),
            tile_config_script: id_static("automancy", "tile_config_script").to_id(interner),
            tile_config_storage: id_static("automancy", "tile_config_storage").to_id(interner),
            tile_config_target: id_static("automancy", "tile_config_target").to_id(interner),
            error_popup: id_static("automancy", "error_popup").to_id(interner),
            debug_menu: id_static("automancy", "debug_menu").to_id(interner),
            load_map: id_static("automancy", "load_map").to_id(interner),
            delete_map: id_static("automancy", "delete_map").to_id(interner),
            create_map: id_static("automancy", "create_map").to_id(interner),
            options: id_static("automancy", "options").to_id(interner),

            lbl_amount: id_static("automancy", "error_popup").to_id(interner),
            lbl_link_destination: id_static("automancy", "lbl_link_destination").to_id(interner),
            lbl_maps_loaded: id_static("automancy", "lbl_maps_loaded").to_id(interner),
            lbl_delete_map_confirm: id_static("automancy", "lbl_delete_map_confirm")
                .to_id(interner),

            btn_confirm: id_static("automancy", "btn_confirm").to_id(interner),
            btn_exit: id_static("automancy", "btn_exit").to_id(interner),
            btn_cancel: id_static("automancy", "btn_cancel").to_id(interner),
            btn_link_network: id_static("automancy", "btn_link_network").to_id(interner),
            btn_play: id_static("automancy", "btn_play").to_id(interner),
            btn_options: id_static("automancy", "btn_options").to_id(interner),
            btn_fedi: id_static("automancy", "btn_fedi").to_id(interner),
            btn_source: id_static("automancy", "btn_source").to_id(interner),
            btn_unpause: id_static("automancy", "btn_unpause").to_id(interner),
            btn_load: id_static("automancy", "btn_load").to_id(interner),
            btn_delete: id_static("automancy", "btn_delete").to_id(interner),
            btn_new_map: id_static("automancy", "btn_new_map").to_id(interner),

            time_fmt: id_static("automancy", "time_fmt").to_id(interner),
        }
    }
}

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
    pub model_ids: ModelIds,
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
