use automancy_data::id_map::IdSet;

use crate::*;

/// The state of the main game GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    MainMenu,
    MapLoad,
    Options(OptionsMenuState),
    Ingame,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionsMenuState {
    #[default]
    Graphics,
    Audio,
    Gui,
    Controls,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PopupState {
    #[default]
    None,
    MapCreate,
    MapDeleteConfirmation(String),
    InvalidName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enum_map::Enum)]
pub enum TextField {
    Filter,
    MapName,
}

#[derive(Clone)]
pub struct TextFieldState {
    fields: enum_map::EnumMap<TextField, String>,
}

impl Debug for TextFieldState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextFieldState")
            .field("fields", &self.fields)
            .finish_non_exhaustive()
    }
}

impl Default for TextFieldState {
    fn default() -> Self {
        TextFieldState {
            fields: enum_map::enum_map! {
                TextField::Filter => Default::default(),
                TextField::MapName => Default::default(),
            },
        }
    }
}

impl TextFieldState {
    pub fn get(&mut self, field: TextField) -> &mut String {
        &mut self.fields[field]
    }

    pub fn take(&mut self, field: TextField) -> String {
        std::mem::replace(&mut self.fields[field], "".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub screen: Screen,
    pub previous: Option<Screen>,
    pub popup: PopupState,

    pub text_field: TextFieldState,
    pub input_hints: Vec<Vec<ActionType>>,

    pub tile_selection_category: Option<CategoryId>,
    /// the currently selected tile.
    pub selected_tile_id: Option<TileId>,
    /// the last placed tile, to prevent repeatedly sending place requests
    pub last_placed_at: Option<TileCoord>,
    /// tile currently linking
    pub linking_tile: Option<(TileCoord, Id, TileEntry)>,
    /// the currently grouped tiles
    pub grouped_tiles: BTreeSet<TileCoord>,
    /// the stored initial cursor position, for moving/copying tiles
    pub paste_from: Option<TileCoord>,
    pub paste_content: FlatTiles,
    pub paste_content_render_cache: BTreeMap<TileCoord, Option<(TileId, Vec<ModelId>)>>,

    pub debugger_open: bool,
    pub tile_config_ui_state: Movable,
    pub player_ui_state: Movable,
    pub debugger_ui_state: Movable,

    pub force_show_puzzle: bool,
    pub selected_research: Option<ResearchId>,
    pub selected_research_puzzle_tile: Option<TileCoord>,
    pub research_puzzle_selections: Option<(TileCoord, IdSet<ModelId>)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Default::default(),
            previous: Default::default(),
            popup: Default::default(),

            text_field: Default::default(),
            input_hints: Default::default(),

            tile_selection_category: Default::default(),
            selected_tile_id: Default::default(),
            last_placed_at: Default::default(),
            linking_tile: Default::default(),
            grouped_tiles: Default::default(),
            paste_from: Default::default(),
            paste_content: Default::default(),
            paste_content_render_cache: Default::default(),

            debugger_open: false,
            tile_config_ui_state: Movable::new(Vec2::new(0.5, 0.5)),
            player_ui_state: Movable::new(Vec2::new(0.5, 0.5)),
            debugger_ui_state: Movable::new(Vec2::new(0.5, 0.5)),

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

    pub fn switch_screen_if(&mut self, new: Screen, when: &'static impl Fn(&UiState) -> bool) -> bool {
        if when(self) {
            self.switch_screen(new);

            true
        } else {
            false
        }
    }
}
