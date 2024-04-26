use enum_map::{enum_map, Enum, EnumMap};
use fuzzy_matcher::skim::SkimMatcherV2;
use hashbrown::{HashMap, HashSet};
use once_cell::sync::Lazy;
use std::fmt::Debug;
use std::sync::Arc;
use std::{collections::BTreeMap, mem};
use tokio::sync::oneshot;
use wgpu::{util::DrawIndexedIndirectArgs, Device, Queue};
use winit::{event_loop::EventLoopWindowTarget, window::Window};
use yakui_wgpu::YakuiWgpu;
use yakui_winit::YakuiWinit;

use automancy_defs::glam::{dvec2, vec3};
use automancy_defs::id::Id;
use automancy_defs::math::Vec2;
use automancy_defs::math::{Float, Matrix4, FAR, HEX_GRID_LAYOUT};
use automancy_defs::rendering::{make_line, InstanceData};
use automancy_defs::{colors, math, window};
use automancy_defs::{coord::TileCoord, glam::vec2};
use automancy_resources::data::Data;
use automancy_resources::data::DataMap;
use automancy_resources::ResourceManager;
use yakui::{
    font::{Font, Fonts},
    widgets::{Absolute, Layer},
    Alignment, Dim2, Pivot, Yakui,
};

use crate::gpu::{AnimationMap, GlobalBuffers, GuiResources};
use crate::input::ActionType;
use crate::GameState;

mod components;

pub use self::components::*;

pub mod debug;
pub mod error;
pub mod info;
pub mod item;
pub mod menu;
pub mod player;
pub mod popup;
pub mod tile_config;
pub mod tile_selection;
pub mod util;

pub const SMALL_ICON_SIZE: Float = 24.0;
pub const SMALLISH_ICON_SIZE: Float = 36.0;
pub const MEDIUM_ICON_SIZE: Float = 48.0;
pub const LARGE_ICON_SIZE: Float = 96.0;

pub struct Gui {
    pub renderer: YakuiWgpu,
    pub yak: Yakui,
    pub window: YakuiWinit,
    pub fonts: HashMap<String, Lazy<Font, Box<dyn FnOnce() -> Font>>>,
    pub font_names: BTreeMap<String, String>,
}

impl Gui {
    pub fn set_font(&mut self, symbols_font: &str, font: &str) {
        let fonts = self.yak.dom().get_global_or_init(Fonts::default);

        fonts.add(
            (*self.fonts.get(symbols_font).unwrap()).clone(),
            Some("symbols"),
        );
        fonts.add((*self.fonts.get(font).unwrap()).clone(), Some("default"));
    }

    pub fn new(device: &Device, queue: &Queue, window: &Window) -> Self {
        let renderer = yakui_wgpu::YakuiWgpu::new(device, queue);
        let window = yakui_winit::YakuiWinit::new(window);
        let yak = Yakui::new();

        Self {
            renderer,
            yak,
            window,
            fonts: Default::default(),
            font_names: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct GuiState {
    pub screen: Screen,
    pub previous: Option<Screen>,
    pub substate: SubState,
    pub popup: PopupState,

    pub debugger_open: bool,

    pub text_field: TextFieldState,

    pub renaming_map: Option<String>,

    pub tile_selection_category: Option<Id>,

    /// the currently selected tile.
    pub selected_tile_id: Option<Id>,
    /// the last placed tile, to prevent repeatedly sending place requests
    pub already_placed_at: Option<TileCoord>,
    /// the tile that has its config menu open.
    pub config_open_at: Option<TileCoord>,
    /// tile currently linking
    pub linking_tile: Option<TileCoord>,
    /// the currently grouped tiles
    pub grouped_tiles: HashSet<TileCoord>,
    /// the stored initial cursor position, for moving/copying tiles
    pub paste_from: Option<TileCoord>,
    pub paste_content: Vec<(TileCoord, Id, Option<DataMap>)>,

    pub placement_direction: Option<TileCoord>,
    pub prev_placement_direction: Option<TileCoord>,

    pub tile_config_ui_position: Vec2,
    pub player_ui_position: Vec2,

    pub force_show_puzzle: bool,
    pub selected_research: Option<Id>,
    pub selected_research_puzzle_tile: Option<TileCoord>,
    pub research_puzzle_selections: Option<(TileCoord, Vec<Id>)>,
}

impl Default for GuiState {
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
            already_placed_at: Default::default(),
            config_open_at: Default::default(),

            linking_tile: Default::default(),
            grouped_tiles: Default::default(),
            paste_from: Default::default(),
            paste_content: Default::default(),
            placement_direction: Default::default(),
            prev_placement_direction: Default::default(),

            tile_config_ui_position: vec2(0.1, 0.1), // TODO make default pos screen center?
            player_ui_position: vec2(0.1, 0.1),

            force_show_puzzle: false,
            selected_research: Default::default(),
            selected_research_puzzle_tile: Default::default(),
            research_puzzle_selections: Default::default(),
        }
    }
}

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

impl GuiState {
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

    pub fn switch_screen_when(
        &mut self,
        when: &'static impl Fn(&GuiState) -> bool,
        new: Screen,
    ) -> bool {
        if when(self) {
            self.switch_screen(new);

            true
        } else {
            false
        }
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Enum, Clone, Copy, Debug)]
pub enum TextField {
    Filter,
    MapRenaming,
    MapName,
}

pub struct TextFieldState {
    pub fuse: SkimMatcherV2,
    fields: EnumMap<TextField, String>,
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
            fuse: SkimMatcherV2::default().ignore_case(),
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

pub type YakuiRenderResources = (
    Arc<ResourceManager>,
    Arc<GlobalBuffers>,
    Option<GuiResources>,
    AnimationMap,
    Option<Vec<(InstanceData, Id, usize)>>,
    HashMap<Id, Vec<(DrawIndexedIndirectArgs, usize)>>,
);

pub fn render_ui(
    state: &mut GameState,
    result: &mut anyhow::Result<bool>,
    target: &EventLoopWindowTarget<()>,
) {
    if state.gui_state.popup == PopupState::None {
        match state.gui_state.screen {
            Screen::Ingame => {
                // tile_info
                info::info_ui(state);

                if !state.input_handler.key_active(ActionType::ToggleGui) {
                    if let Some(map_info) = state.loop_store.map_info.as_ref().map(|v| v.0.clone())
                    {
                        let mut lock = map_info.blocking_lock();
                        let game_data = &mut lock.data;

                        let (selection_send, selection_recv) = oneshot::channel();

                        // tile_config
                        tile_config::tile_config_ui(state, game_data);

                        // tile_selections
                        tile_selection::tile_selections(state, game_data, selection_send);

                        if let Ok(id) = selection_recv.blocking_recv() {
                            state.gui_state.already_placed_at = None;

                            if state.gui_state.selected_tile_id == Some(id) {
                                state.gui_state.selected_tile_id = None;
                            } else {
                                state.gui_state.selected_tile_id = Some(id);
                            }
                        }

                        if state.input_handler.key_active(ActionType::Player) {
                            player::player(state, game_data);
                        }
                    }

                    let cursor_pos = math::screen_to_world(
                        window::window_size_double(&state.renderer.gpu.window),
                        state.input_handler.main_pos,
                        state.camera.get_pos(),
                    );
                    let cursor_pos = dvec2(cursor_pos.x, cursor_pos.y);

                    if let Some(tile_def) = state
                        .gui_state
                        .selected_tile_id
                        .and_then(|id| state.resource_man.registry.tiles.get(&id))
                    {
                        Absolute::new(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Dim2::ZERO).show(
                            || {
                                ui_game_object(
                                    InstanceData::default()
                                        .with_alpha(0.6)
                                        .with_light_pos(state.camera.get_pos().as_vec3(), None)
                                        .with_world_matrix(state.camera.get_matrix().as_mat4())
                                        .with_model_matrix(Matrix4::from_translation(vec3(
                                            cursor_pos.x as Float,
                                            cursor_pos.y as Float,
                                            FAR as Float,
                                        ))),
                                    tile_def.model,
                                    state.gui.yak.layout_dom().viewport().size(),
                                );
                            },
                        );
                    }

                    if let Some(coord) = state.gui_state.linking_tile {
                        state.renderer.extra_instances.push((
                            InstanceData::default()
                                .with_color_offset(colors::RED.to_linear())
                                .with_light_pos(state.camera.get_pos().as_vec3(), None)
                                .with_world_matrix(state.camera.get_matrix().as_mat4())
                                .with_model_matrix(make_line(
                                    HEX_GRID_LAYOUT.hex_to_world_pos(*coord),
                                    cursor_pos.as_vec2(),
                                )),
                            state.resource_man.registry.model_ids.cube1x1,
                        ));
                    }

                    if let Some((dir, selected_tile_id)) = state
                        .gui_state
                        .placement_direction
                        .zip(state.gui_state.selected_tile_id)
                    {
                        if dir != TileCoord::ZERO
                            && !state.resource_man.registry.tiles[&selected_tile_id]
                                .data
                                .get(&state.resource_man.registry.data_ids.indirectional)
                                .cloned()
                                .and_then(Data::into_bool)
                                .unwrap_or(false)
                        {
                            state.renderer.extra_instances.push((
                                InstanceData::default()
                                    .with_color_offset(colors::RED.to_linear())
                                    .with_light_pos(state.camera.get_pos().as_vec3(), None)
                                    .with_world_matrix(state.camera.get_matrix().as_mat4())
                                    .with_model_matrix(make_line(
                                        HEX_GRID_LAYOUT.hex_to_world_pos(*state.camera.pointing_at),
                                        HEX_GRID_LAYOUT
                                            .hex_to_world_pos(*(state.camera.pointing_at + dir)),
                                    )),
                                state.resource_man.registry.model_ids.cube1x1,
                            ));
                        }
                    }
                }
            }
            Screen::MainMenu => *result = menu::main_menu(state, target),
            Screen::MapLoad => {
                menu::map_menu(state);
            }
            Screen::Options => {
                menu::options_menu(state);
            }
            Screen::Paused => {
                menu::pause_menu(state);
            }
        }
    }

    match state.gui_state.popup.clone() {
        PopupState::None => {}
        PopupState::MapCreate => popup::map_create_popup(state),
        PopupState::MapDeleteConfirmation(map_name) => {
            popup::map_delete_popup(state, &map_name);
        }
        PopupState::InvalidName => {
            popup::invalid_name_popup(state);
        }
    }

    render_info_tip(state);

    state.renderer.tile_tints.insert(
        state.camera.pointing_at,
        colors::RED.with_alpha(0.2).to_linear(),
    );

    for coord in &state.gui_state.grouped_tiles {
        state
            .renderer
            .tile_tints
            .insert(*coord, colors::ORANGE.with_alpha(0.4).to_linear());
    }

    if let Some(start) = state.gui_state.paste_from {
        if start != state.camera.pointing_at {
            state.renderer.extra_instances.push((
                InstanceData::default()
                    .with_color_offset(colors::LIGHT_BLUE.to_linear())
                    .with_light_pos(state.camera.get_pos().as_vec3(), None)
                    .with_world_matrix(state.camera.get_matrix().as_mat4())
                    .with_model_matrix(make_line(
                        HEX_GRID_LAYOUT.hex_to_world_pos(*start),
                        HEX_GRID_LAYOUT.hex_to_world_pos(*state.camera.pointing_at),
                    )),
                state.resource_man.registry.model_ids.cube1x1,
            ));
        }

        let diff = state.camera.pointing_at - start;

        for (coord, id, data) in &state.gui_state.paste_content {
            let coord = *coord + diff;
            let p = HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

            let mut model_matrix = Matrix4::from_translation(vec3(p.x, p.y, FAR as Float));

            if let Some(data) = data {
                let rotate = Matrix4::from_rotation_z(
                    data.get(&state.resource_man.registry.data_ids.direction)
                        .and_then(|direction| {
                            if let Data::Coord(direction) = direction {
                                math::tile_direction_to_angle(*direction)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0.0)
                        .to_radians(),
                );

                model_matrix *= rotate;
            }

            Layer::new().show(|| {
                ui_game_object(
                    InstanceData::default()
                        .with_alpha(0.5)
                        .with_light_pos(state.camera.get_pos().as_vec3(), None)
                        .with_world_matrix(state.camera.get_matrix().as_mat4())
                        .with_model_matrix(model_matrix),
                    state.resource_man.registry.tiles[id].model,
                    state.gui.yak.layout_dom().viewport().size(),
                );
            });
        }
    }

    if state.input_handler.key_active(ActionType::Debug) {
        debug::debugger(state);
    }

    error::error_popup(state);
}
