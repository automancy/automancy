use automancy_defs::graph::graph::NodeIndex;
use automancy_defs::graph::prelude::StableDiGraph;
use automancy_defs::id::Id;
use automancy_macros::IdReg;
use hashbrown::HashMap;

use crate::data::item::Item;
use crate::types::category::Category;
use crate::types::research::Research;
use crate::types::script::Script;
use crate::types::tag::Tag;
use crate::types::tile::TileDef;

/// Represents the resource registry.
#[derive(Clone)]
pub struct Registry {
    pub tiles: HashMap<Id, TileDef>,
    pub scripts: HashMap<Id, Script>,
    pub tags: HashMap<Id, Tag>,
    pub categories: HashMap<Id, Category>,
    pub items: HashMap<Id, Item>,
    pub researches: StableDiGraph<Research, ()>,
    pub researches_id_map: HashMap<Id, NodeIndex>,
    pub researches_unlock_map: HashMap<Id, NodeIndex>,

    pub none: Id,
    pub any: Id,

    pub data_ids: DataIds,
    pub model_ids: ModelIds,
    pub gui_ids: GuiIds,
    pub err_ids: ErrorIds,
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
    pub player_inventory: Id,

    pub direction_color: Id,
    pub storage_takeable: Id,
    pub inactive_model: Id,
    pub not_targeted: Id,
    pub max_amount: Id,
    pub linked: Id,
    pub linking: Id,
    pub default_tile: Id,
    pub unlocked_researches: Id,
    pub category: Id,
}

#[derive(Copy, Clone, IdReg)]
pub struct ModelIds {
    #[namespace(core)]
    pub missing: Id,
    #[namespace(core)]
    pub items_missing: Id,
    #[namespace(core)]
    pub cube1x1: Id,
}

/// The list of GUI translation keys.
#[derive(Clone, Copy, IdReg)]
pub struct GuiIds {
    pub info: Id,
    pub player_menu: Id,
    pub player_inventory: Id,
    pub open_research: Id,
    pub tile_config: Id,
    pub tile_config_script: Id,
    pub tile_config_script_info: Id,
    pub tile_config_item: Id,
    pub tile_config_target: Id,
    pub error_popup: Id,
    pub debug_menu: Id,
    pub load_map: Id,
    pub delete_map: Id,
    pub create_map: Id,
    pub invalid_name: Id,
    pub options: Id,

    pub hint_search_script: Id,
    pub hint_search_item: Id,

    pub lbl_amount: Id,
    pub lbl_link_destination: Id,
    pub lbl_maps_loaded: Id,
    pub lbl_pick_another_name: Id,
    pub lbl_delete_map_confirm: Id,
    pub lbl_cannot_place_missing_item: Id,

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
