use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_macros::IdReg;

use crate::data::item::Item;
use crate::data::Data;
use crate::script::Script;
use crate::tag::Tag;
use crate::tile::Tile;

/// Represents the resource registry.
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
    pub gui_ids: GuiIds,
    pub err_ids: ErrorIds,
}

impl Registry {
    pub fn tile_data(&self, id: Id, data: Id) -> Option<&Data> {
        self.tiles.get(&id).and_then(|v| v.data.get(&data))
    }

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

#[derive(Copy, Clone, IdReg)]
pub struct DataIds {
    pub script: Id,
    pub scripts: Id,
    pub buffer: Id,
    pub item: Id,
    pub item_type: Id,
    pub amount: Id,
    pub target: Id,
    pub link: Id,

    pub inactive_model: Id,
    pub not_targeted: Id,
    pub max_amount: Id,
    pub linked: Id,
    pub linking: Id,
}

#[derive(Copy, Clone, IdReg)]
pub struct ModelIds {
    #[namespace(core)]
    pub missing: Id,
    #[namespace(core)]
    pub items_missing: Id,
}

/// The list of GUI translation keys.
#[derive(Clone, Copy, IdReg)]
pub struct GuiIds {
    pub tile_config: Id,
    pub tile_info: Id,
    pub tile_config_script: Id,
    pub tile_config_item: Id,
    pub tile_config_target: Id,
    pub error_popup: Id,
    pub debug_menu: Id,
    pub load_map: Id,
    pub delete_map: Id,
    pub create_map: Id,
    pub invalid_name: Id,
    pub options: Id,

    pub lbl_amount: Id,
    pub lbl_link_destination: Id,
    pub lbl_maps_loaded: Id,
    pub lbl_pick_another_name: Id,
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

/// Contains a list of errors that can be displayed.
#[derive(Clone, Copy, IdReg)]
pub struct ErrorIds {
    /// This error is displayed when the map cannot be read.
    pub invalid_map_data: Id,
    /// This error is displayed when the options cannot be written.
    pub unwritable_options: Id,
}
