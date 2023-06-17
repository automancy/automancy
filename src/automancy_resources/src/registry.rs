use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::{id_static, Id, Interner};
use automancy_macros::make_ids;

use crate::data::item::Item;
use crate::script::Script;
use crate::tag::Tag;
use crate::tile::Tile;

/// Represents the automancy_resources registry.
#[derive(Clone)]
pub struct Registry {
    pub tiles: HashMap<Id, Tile>,
    pub scripts: HashMap<Id, Script>,
    pub tags: HashMap<Id, Tag>,
    pub items: HashMap<Id, Item>,

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
        make_ids! {
            machine,
            transfer,
            void,
            storage,
            merger,
            splitter,
            master_node,
            node,
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
        make_ids! {
            script,
            scripts,
            buffer,
            storage,
            storage_type,
            amount,
            target,
            link,
        }
    }
}

#[derive(Copy, Clone)]
pub struct ModelIds {
    pub items_missing: Id,
}

impl ModelIds {
    pub fn new(interner: &mut Interner) -> Self {
        make_ids! {
            items_missing
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
        make_ids! {
            tile_config,
            tile_info,
            tile_config_script,
            tile_config_storage,
            tile_config_target,
            error_popup,
            debug_menu,
            load_map,
            delete_map,
            create_map,
            options,

            lbl_amount,
            lbl_link_destination,
            lbl_maps_loaded,
            lbl_delete_map_confirm,

            btn_confirm,
            btn_exit,
            btn_cancel,
            btn_link_network,
            btn_play,
            btn_options,
            btn_fedi,
            btn_source,
            btn_unpause,
            btn_load,
            btn_delete,
            btn_new_map,

            time_fmt,
        }
    }
}

/// Contains a list of errors that can be displayed.
#[derive(Clone, Copy)]
pub struct ErrorIds {
    /// This error is displayed to test that the error manager is working. TODO this can probably be removed.
    pub test_error: Id,
    /// This error is displayed when the map cannot be read.
    pub invalid_map_data: Id,
}

impl ErrorIds {
    pub fn new(interner: &mut Interner) -> Self {
        make_ids! {
            test_error
            invalid_map_data
        }
    }
}
