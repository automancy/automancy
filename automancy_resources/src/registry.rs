use automancy_defs::graph::graph::NodeIndex;
use automancy_defs::graph::prelude::StableDiGraph;
use automancy_defs::id::Id;
use automancy_macros::IdReg;
use hashbrown::HashMap;

use crate::types::research::ResearchDef;
use crate::types::script::ScriptDef;
use crate::types::tag::TagDef;
use crate::types::tile::TileDef;
use crate::types::{category::CategoryDef, item::ItemDef};

/// Represents the resource registry.
#[derive(Clone)]
pub struct Registry {
    pub tiles: HashMap<Id, TileDef>,
    pub scripts: HashMap<Id, ScriptDef>,
    pub tags: HashMap<Id, TagDef>,
    pub categories: HashMap<Id, CategoryDef>,
    pub items: HashMap<Id, ItemDef>,
    pub researches: StableDiGraph<ResearchDef, ()>,
    pub researches_id_map: HashMap<Id, NodeIndex>,
    pub researches_unlock_map: HashMap<Id, NodeIndex>,

    pub none: Id,
    pub any: Id,

    pub data_ids: DataIds,
    pub model_ids: ModelIds,
    pub gui_ids: GuiIds,
    pub key_ids: KeyIds,
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
    pub direction: Id,
    pub link: Id,

    pub player_inventory: Id,
    pub research_items_filled: Id,
    pub research_puzzle_completed: Id,
    pub tiles: Id,

    pub direction_color: Id,
    pub storage_takeable: Id,
    pub inactive_model: Id,
    pub indirectional: Id,
    pub linked: Id,
    pub linking: Id,
    pub default_tile: Id,
    pub unlocked_researches: Id,
    pub category: Id,
}

#[derive(Copy, Clone, IdReg)]
pub struct ModelIds {
    #[namespace("core")]
    pub missing: Id,
    #[namespace("core")]
    pub items_missing: Id,
    #[namespace("core")]
    pub cube1x1: Id,
    #[namespace("core")]
    pub puzzle_space: Id,
}

#[derive(Clone, Copy, IdReg)]
pub struct GuiIds {
    pub info: Id,
    pub player_menu: Id,
    pub error_popup: Id,
    pub debug_menu: Id,
    pub load_map: Id,
    pub delete_map: Id,
    pub create_map: Id,
    pub invalid_name: Id,
    pub options: Id,
    pub inventory: Id,
    pub tile_config: Id,

    pub inventory_tip: Id,
    pub search_tip: Id,
    pub search_script_tip: Id,
    pub search_item_tip: Id,
    pub direction_tip: Id,
    pub link_destination_tip: Id,

    pub tile_config_script_info: Id,
    pub tile_config_script: Id,
    pub tile_config_item_type: Id,
    pub tile_config_direction: Id,

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

    pub research_menu_title: Id,
    pub player_inventory_title: Id,
    pub research_submit_items: Id,

    pub time_fmt: Id,
}

// The list of GUI translation keys.
#[derive(Clone, Copy, IdReg)]
pub struct KeyIds {
    pub cancel: Id,
    pub pause: Id,
    pub undo: Id,
    pub redo: Id,
    pub toggle_gui: Id,
    pub player_menu: Id,
    pub remove_tile: Id,
    pub select_mode: Id,
    pub hotkey: Id,
    pub cut: Id,
    pub copy: Id,
    pub paste: Id,
}

#[derive(Clone, Copy, IdReg)]
pub struct ErrorIds {
    /// This error is displayed when the map cannot be read.
    #[namespace("core")]
    pub invalid_map_data: Id,
    /// This error is displayed when the options cannot be written.
    #[namespace("core")]
    pub unwritable_options: Id,
}
