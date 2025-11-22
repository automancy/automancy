use core::{fmt::Debug, mem};

use automancy_data::{
    game::coord::TileCoord,
    id::{Id, ModelId, TileId},
    math::Vec2,
};
use enum_map::{Enum, enum_map};
use hashbrown::{HashMap, HashSet};

use crate::actor::{FlatTiles, TileEntry};

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum Screen {
    #[default]
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum SubState {
    #[default]
    None,
    Options(OptionsMenuState),
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum OptionsMenuState {
    Graphics,
    Audio,
    Gui,
    Controls,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub enum PopupState {
    #[default]
    None,
    MapCreate,
    MapDeleteConfirmation(String),
    InvalidName,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Enum)]
pub enum TextField {
    Filter,
    MapRenaming,
    MapName,
}

pub struct TextFieldState {
    pub fuse: fuzzy_matcher::skim::SkimMatcherV2,
    fields: enum_map::EnumMap<TextField, String>,
}

impl Debug for TextFieldState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextFieldState").field("fields", &self.fields).finish_non_exhaustive()
    }
}

impl Default for TextFieldState {
    fn default() -> Self {
        TextFieldState {
            fuse: fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case(),
            fields: enum_map! {
                TextField::Filter => Default::default(),
                TextField::MapName => Default::default(),
                TextField::MapRenaming => Default::default()
            },
        }
    }
}

impl TextFieldState {
    pub fn get(&mut self, field: TextField) -> &mut String {
        &mut self.fields[field]
    }

    pub fn take(&mut self, field: TextField) -> String {
        mem::replace(&mut self.fields[field], "".to_string())
    }
}

#[derive(Debug)]
pub struct UiState {
    pub screen: Screen,
    pub previous: Option<Screen>,
    pub substate: SubState,
    pub popup: PopupState,

    pub debugger_open: bool,

    pub text_field: TextFieldState,

    pub renaming_map: Option<String>,

    pub tile_selection_category: Option<Id>,

    /// the currently selected tile.
    pub selected_tile_id: Option<TileId>,
    /// the currently selected tile's model ids.
    pub selected_tile_render_cache: Option<(TileId, Vec<ModelId>)>,
    /// the last placed tile, to prevent repeatedly sending place requests
    pub last_placed_at: Option<TileCoord>,
    /// the tile that has its config menu open.
    pub config_open_at: Option<TileCoord>,
    /// tile currently linking
    pub linking_tile: Option<(Id, TileEntry)>,
    /// the currently grouped tiles
    pub grouped_tiles: HashSet<TileCoord>,
    /// the stored initial cursor position, for moving/copying tiles
    pub paste_from: Option<TileCoord>,
    pub paste_content: FlatTiles,
    pub paste_content_render_cache: HashMap<TileCoord, Option<(TileId, Vec<ModelId>)>>,

    pub tile_config_ui_position: Vec2,
    pub player_ui_position: Vec2,
    pub debugger_ui_position: Vec2,

    pub force_show_puzzle: bool,
    pub selected_research: Option<Id>,
    pub selected_research_puzzle_tile: Option<TileCoord>,
    pub research_puzzle_selections: Option<(TileCoord, Vec<Id>)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Default::default(),
            previous: Default::default(),
            substate: Default::default(),
            popup: Default::default(),
            debugger_open: Default::default(),
            text_field: Default::default(),
            renaming_map: Default::default(),
            tile_selection_category: Default::default(),

            selected_tile_id: Default::default(),
            selected_tile_render_cache: Default::default(),
            last_placed_at: Default::default(),
            config_open_at: Default::default(),

            linking_tile: Default::default(),
            grouped_tiles: Default::default(),
            paste_from: Default::default(),
            paste_content: Default::default(),
            paste_content_render_cache: HashMap::new(),

            tile_config_ui_position: Vec2::new(0.1, 0.1), // TODO make default pos screen center?
            player_ui_position: Vec2::new(0.1, 0.1),
            debugger_ui_position: Vec2::new(0.1, 0.1),

            force_show_puzzle: false,
            selected_research: Default::default(),
            selected_research_puzzle_tile: Default::default(),
            research_puzzle_selections: Default::default(),
        }
    }
}

impl UiState {
    pub fn return_screen(&mut self) {
        if let Some(prev) = self.previous {
            self.screen = prev;
        }
        self.previous = None;
    }

    pub fn switch_screen(&mut self, new: Screen) {
        self.previous = Some(self.screen);
        self.screen = new;
    }

    pub fn switch_screen_sub(&mut self, new: Screen, sub: SubState) {
        self.switch_screen(new);
        self.substate = sub;
    }

    pub fn switch_screen_if(&mut self, new: Screen, when: &'static impl Fn(&UiState) -> bool) -> bool {
        if when(self) {
            self.switch_screen(new);

            true
        } else {
            false
        }
    }
}
