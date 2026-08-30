use automancy_data::{
    game::generic::DataMap,
    id::{
        CategoryId, GuiTranslateId, Id, IdInterner, ItemId, ModelId, RecipeId, RenderId, ResearchId, ScriptId, TagId, TileId, UiRenderId,
    },
    id_map::{IdMap, ImmutableIdMap},
    rendering::GenericModel,
};
use automancy_macros::IdReg;
use petgraph::{graph::NodeIndex, prelude::StableDiGraph};

use crate::resources::types::{category::CategoryDef, item::ItemDef, recipe::RecipeDef, research::ResearchDef, tag::TagDef, tile::TileDef};

/// Represents a "Resource Registry", which stores all the definition files and preset Ids.
pub struct Registry {
    pub tile_defs: ImmutableIdMap<TileId, TileDef>,
    pub item_defs: ImmutableIdMap<ItemId, ItemDef>,
    pub recipe_defs: ImmutableIdMap<RecipeId, RecipeDef>,
    pub tag_defs: ImmutableIdMap<TagId, TagDef>,
    pub category_defs: ImmutableIdMap<CategoryId, CategoryDef>,
    pub research_defs: StableDiGraph<ResearchDef, ()>,
    pub(crate) research_id_map: ImmutableIdMap<ResearchId, NodeIndex>,

    pub data_ids: DataIds,
    pub model_ids: ModelIds,
    pub gui_ids: GuiIds,
    pub key_ids: KeyIds,
    pub err_ids: ErrorIds,
    pub render_ids: RenderIds,

    pub(crate) category_tiles_map: ImmutableIdMap<CategoryId, Vec<TileId>>,
    pub(crate) research_unlock_map: ImmutableIdMap<TileId, NodeIndex>,
}

/// Mutable version of [`Registry`].
pub struct MutableRegistry {
    pub(crate) tile_defs: IdMap<TileId, TileDef>,
    pub(crate) item_defs: IdMap<ItemId, ItemDef>,
    pub(crate) recipe_defs: IdMap<RecipeId, RecipeDef>,
    pub(crate) tag_defs: IdMap<TagId, TagDef>,
    pub(crate) category_defs: IdMap<CategoryId, CategoryDef>,
    pub(crate) research_defs: StableDiGraph<ResearchDef, ()>,
    pub(crate) research_id_map: IdMap<ResearchId, NodeIndex>,

    pub(crate) data_ids: DataIds,
    pub(crate) model_ids: ModelIds,
    pub(crate) gui_ids: GuiIds,
    pub(crate) key_ids: KeyIds,
    pub(crate) err_ids: ErrorIds,
    pub(crate) render_ids: RenderIds,
}

impl MutableRegistry {
    pub fn new(interner: &mut IdInterner) -> MutableRegistry {
        let data_ids = DataIds::new(interner);
        let model_ids = ModelIds::new(interner);
        let gui_ids = GuiIds::new(interner);
        let key_ids = KeyIds::new(interner);
        let err_ids = ErrorIds::new(interner);
        let render_ids = RenderIds::new(interner);

        MutableRegistry {
            tile_defs: IdMap::from_iter([(
                TileId::none(),
                TileDef {
                    id: TileId::none(),
                    script: ScriptId::none(),
                    category: CategoryId::none(),
                    data: DataMap::new(),

                    lua_def: mlua::Nil,
                },
            )]),
            item_defs: IdMap::from_iter([(
                ItemId::none(),
                ItemDef {
                    id: ItemId::none(),
                    model: ModelId::none(),
                },
            )]),
            recipe_defs: IdMap::from_iter([(
                RecipeId::none(),
                RecipeDef {
                    id: RecipeId::none(),
                    inputs: None,
                    outputs: Vec::new(),
                },
            )]),
            tag_defs: IdMap::from_iter([
                (
                    TagId::none(),
                    TagDef {
                        id: TagId::none(),
                        entries: Default::default(),
                    },
                ),
                (
                    TagId::any(),
                    TagDef {
                        id: TagId::any(),
                        entries: Default::default(),
                    },
                ),
            ]),
            category_defs: IdMap::from_iter([(
                CategoryId::none(),
                CategoryDef {
                    id: CategoryId::none(),
                    ord: 0,
                    icon: GenericModel::Plain(ModelId::none()),
                    item: ItemId::none(),
                },
            )]),
            research_defs: StableDiGraph::new(),
            research_id_map: IdMap::default(),

            data_ids,
            model_ids,
            gui_ids,
            err_ids,
            key_ids,
            render_ids,
        }
    }
}

#[derive(Clone, Copy, IdReg)]
pub struct DataIds {
    pub recipe: Id,
    pub item: Id,
    pub capacity: Id,
    pub direction: Id,
    pub link: Id,

    pub player_inventory: Id,
    pub research_items_filled: Id,
    pub research_puzzle_completed: Id,

    #[namespace("core")]
    pub research_board_tiles: Id,
    #[namespace("core")]
    pub unlocked_researches: Id,
    #[namespace("core")]
    pub default_tile: Id,
    #[namespace("core")]
    pub debug_unlock_everything: Id,
}

#[derive(Clone, Copy, IdReg)]
pub struct ModelIds {
    #[namespace("core")]
    #[name("tile/none")]
    pub tile_none: ModelId,

    #[namespace("core")]
    #[name("tile/missing")]
    pub tile_missing: ModelId,
    #[namespace("core")]
    #[name("item/missing")]
    pub item_missing: ModelId,

    #[namespace("core")]
    pub cube1x1: ModelId,
    #[namespace("core")]
    pub puzzle_space: ModelId,
}

#[derive(Clone, Copy, IdReg)]
pub struct GuiIds {
    pub time_fmt: GuiTranslateId,

    pub info_menu_title: GuiTranslateId,
    pub player_menu_title: GuiTranslateId,
    pub debug_menu_title: GuiTranslateId,
    pub tile_config_menu_title: GuiTranslateId,
    pub research_menu_title: GuiTranslateId,
    pub player_inventory_title: GuiTranslateId,

    pub main_menu_title: GuiTranslateId,
    pub main_menu_source_button: GuiTranslateId,
    pub main_menu_discord_button: GuiTranslateId,
    pub main_menu_play_button: GuiTranslateId,
    pub main_menu_exit_button: GuiTranslateId,
    pub main_menu_options_button: GuiTranslateId,

    pub pause_menu_title: GuiTranslateId,
    pub pause_menu_unpause_button: GuiTranslateId,
    pub pause_menu_options_button: GuiTranslateId,
    pub pause_menu_quit_to_menu_button: GuiTranslateId,

    pub map_list_title: GuiTranslateId,
    pub map_list_load_button: GuiTranslateId,
    pub map_list_delete_button: GuiTranslateId,
    pub map_list_new_map_button: GuiTranslateId,
    pub map_list_exit_button: GuiTranslateId,
    pub map_list_total_label: GuiTranslateId,
    pub map_create_title: GuiTranslateId,
    pub map_create_name_input_label: GuiTranslateId,
    pub map_create_name_input_placeholder: GuiTranslateId,
    pub map_create_confirm_button: GuiTranslateId,
    pub map_create_cancel_button: GuiTranslateId,
    pub map_delete_title: GuiTranslateId,
    pub map_delete_label: GuiTranslateId,
    pub map_delete_confirm_button: GuiTranslateId,
    pub map_delete_confirm_warning: GuiTranslateId,
    pub map_delete_confirm_again_button: GuiTranslateId,
    pub map_delete_cancel_button: GuiTranslateId,
    pub map_invalid_name_title: GuiTranslateId,
    pub map_invalid_name_label: GuiTranslateId,
    pub map_invalid_name_accept_button: GuiTranslateId,

    pub tile_missing_item_label: GuiTranslateId,

    pub options_title: GuiTranslateId,
    pub options_finish_button: GuiTranslateId,
    pub options_apply_button: GuiTranslateId,

    pub options_graphics_title: GuiTranslateId,
    pub options_graphics_ui_scale: GuiTranslateId,
    pub options_graphics_ui_scale_small: GuiTranslateId,
    pub options_graphics_ui_scale_normal: GuiTranslateId,
    pub options_graphics_ui_scale_large: GuiTranslateId,
    pub options_graphics_fps: GuiTranslateId,
    pub options_graphics_fps_vsync: GuiTranslateId,
    pub options_graphics_fps_vsync_warning: GuiTranslateId,
    pub options_graphics_fps_unlimited: GuiTranslateId,
    pub options_graphics_fullscreen: GuiTranslateId,
    pub options_graphics_antialiasing: GuiTranslateId,
    pub options_graphics_antialiasing_none: GuiTranslateId,
    pub options_graphics_antialiasing_fxaa: GuiTranslateId,

    pub options_audio_title: GuiTranslateId,
    pub options_audio_sfx_volume: GuiTranslateId,
    pub options_audio_music_volume: GuiTranslateId,

    pub options_gui_title: GuiTranslateId,
    pub options_gui_font: GuiTranslateId,
    pub options_gui_font_system_fonts: GuiTranslateId,
    pub options_gui_font_system_fonts_preferences: GuiTranslateId,
    pub options_gui_language: GuiTranslateId,

    pub options_controls_title: GuiTranslateId,

    pub error_menu_title: GuiTranslateId,
    pub error_menu_label: GuiTranslateId,
    pub error_menu_error_message: GuiTranslateId,
    pub error_menu_error_id: GuiTranslateId,
    pub error_menu_read_button: GuiTranslateId,

    pub tile_config_recipe_outputs: GuiTranslateId,
    pub tile_config_recipe_inputs_and_outputs: GuiTranslateId,

    pub btn_research_submit_items: GuiTranslateId,
}

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

#[derive(Clone, Copy, IdReg)]
pub struct RenderIds {
    #[namespace("core")]
    pub empty_tiles: RenderId,

    #[namespace("core")]
    pub overlay: RenderId,

    #[namespace("core")]
    pub overlay_z_zero_floor: UiRenderId,

    #[namespace("core")]
    pub selected_tile: UiRenderId,

    #[namespace("core")]
    pub linking_line: UiRenderId,

    #[namespace("core")]
    pub pasting_line: UiRenderId,

    #[namespace("core")]
    pub pasting_content: UiRenderId,

    #[namespace("core")]
    pub player_inventory_animation: UiRenderId,
}
