use std::mem;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{
    CursorIcon, LayerId, PaintCallbackInfo, Rect, ScrollArea, TextEdit, Ui, Widget, WidgetText,
};
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use enum_map::{enum_map, Enum, EnumMap};
use fuse_rust::Fuse;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use tokio::sync::oneshot;
use wgpu::util::DrawIndexedIndirectArgs;
use wgpu::{CommandBuffer, CommandEncoder, Device, IndexFormat, Queue, RenderPass};
use winit::event_loop::EventLoopWindowTarget;

use automancy_defs::colors::ColorAdj;
use automancy_defs::coord::TileCoord;
use automancy_defs::glam::{dvec2, dvec3, vec3};
use automancy_defs::id::Id;
use automancy_defs::math::{Float, Matrix4, FAR, HEX_GRID_LAYOUT};
use automancy_defs::rendering::{make_line, InstanceData};
use automancy_defs::{bytemuck, colors, math, window};
use automancy_resources::data::item::Item;
use automancy_resources::data::Data;
use automancy_resources::ResourceManager;

use crate::game::TAKE_ITEM_ANIMATION_SPEED;
use crate::gpu::{AnimationMap, GlobalBuffers, GuiResources};
use crate::input::KeyActions;
use crate::renderer::try_add_animation;
use crate::{gpu, GameState};

pub mod debug;
pub mod error;
pub mod info;
pub mod item;
pub mod menu;
pub mod player;
pub mod popup;
pub mod research;
pub mod tile_config;
pub mod tile_selection;

pub const SMALL_ICON_SIZE: Float = 24.0;
pub const MEDIUM_ICON_SIZE: Float = 48.0;
pub const LARGE_ICON_SIZE: Float = 96.0;

pub struct GuiState {
    pub screen: Screen,
    pub previous: Option<Screen>,
    pub substate: SubState,
    pub popup: PopupState,

    pub debugger_open: bool,
    pub research_open: bool,

    pub text_field: TextFieldState,

    pub renaming_map: String,

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
    /// the stored initial cursor position, for moving tiles
    pub initial_cursor_position: Option<TileCoord>,
    /// the current tile placement target direction, only Some when shift is held
    /// TODO shift is only on keyboard
    pub placement_direction: Option<TileCoord>,
    pub prev_placement_direction: Option<TileCoord>,
}

impl Default for GuiState {
    fn default() -> Self {
        GuiState {
            screen: Screen::MainMenu,
            previous: None,
            substate: SubState::None,
            popup: PopupState::None,
            debugger_open: false,
            research_open: false,
            text_field: Default::default(),
            renaming_map: "".to_string(),
            tile_selection_category: None,
            selected_tile_id: None,
            already_placed_at: None,
            config_open_at: None,
            linking_tile: None,
            grouped_tiles: Default::default(),
            initial_cursor_position: None,
            placement_direction: None,
            prev_placement_direction: None,
        }
    }
}

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Screen {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum SubState {
    None,
    Options(OptionsMenuState),
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum OptionsMenuState {
    Graphics,
    Audio,
    Gui,
    Controls,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Eq, PartialEq, Clone)]
pub enum PopupState {
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
        when: &'static dyn Fn(&GuiState) -> bool,
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Enum, Clone, Copy)]
pub enum TextField {
    Filter,
    MapRenaming,
    MapName,
}

pub struct TextFieldState {
    pub fuse: Fuse,
    fields: EnumMap<TextField, String>,
}

impl Default for TextFieldState {
    fn default() -> Self {
        TextFieldState {
            fuse: Fuse::default(),
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

pub fn hover_tip(ui: &mut Ui, info: impl Into<WidgetText>) {
    ui.label("\u{f449}")
        .on_hover_cursor(CursorIcon::Help)
        .on_hover_ui(|ui| {
            ui.label(info);
        });
}

fn take_item_animation(state: &mut GameState, ui: &mut Ui, item: Item, dst_rect: Rect) {
    let now = Instant::now();

    let mut to_remove = HashMap::new();

    for (coord, deque) in &state.renderer.take_item_animations {
        to_remove.insert(
            *coord,
            deque
                .iter()
                .take_while(|(instant, _)| {
                    now.duration_since(*instant) >= TAKE_ITEM_ANIMATION_SPEED
                })
                .count(),
        );
    }

    for (coord, v) in to_remove {
        for _ in 0..v {
            state
                .renderer
                .take_item_animations
                .get_mut(&coord)
                .unwrap()
                .pop_front();
        }
    }

    if let Some(animations) = state.renderer.take_item_animations.get(&item) {
        for (instant, src_rect) in animations {
            let d = now.duration_since(*instant).as_secs_f32()
                / TAKE_ITEM_ANIMATION_SPEED.as_secs_f32();
            let rect = src_rect.lerp_towards(&dst_rect, d);

            ui.ctx()
                .layer_painter(ui.layer_id())
                .add(egui_wgpu::Callback::new_paint_callback(
                    rect,
                    GameEguiCallback::new(
                        InstanceData::default()
                            .with_world_matrix(math::view(dvec3(0.0, 0.0, 1.0)).as_mat4()),
                        state.resource_man.get_item_model(item),
                        rect,
                        ui.ctx().screen_rect(),
                    ),
                ));
        }
    }
}

/// Draws a search bar.
pub fn searchable_id(
    state: &mut GameState,
    ui: &mut Ui,
    ids: &[Id],
    new_id: &mut Option<Id>,
    field: TextField,
    hint_text: impl Into<WidgetText>,
    to_string: &'static impl Fn(&GameState, &Id) -> String,
    draw_item: &'static impl Fn(&mut GameState, &mut Ui, &Id),
) {
    TextEdit::singleline(state.gui_state.text_field.get(field))
        .hint_text(hint_text)
        .ui(ui);

    ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
        ui.set_width(ui.available_width());

        let ids = if !state.gui_state.text_field.get(field).is_empty() {
            let text = state.gui_state.text_field.get(field).clone();
            let mut filtered = ids
                .iter()
                .flat_map(|id| {
                    let result = state
                        .gui_state
                        .text_field
                        .fuse
                        .search_text_in_string(&text, &to_string(state, id));
                    let score = result.map(|v| v.score);

                    if score.unwrap_or(0.0) > 0.4 {
                        None
                    } else {
                        Some(*id).zip(score)
                    }
                })
                .collect::<Vec<_>>();
            filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
        } else {
            ids.to_vec()
        };

        for id in ids {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.interact_size.y = SMALL_ICON_SIZE;

                ui.radio_value(new_id, Some(id), format!("{}:", to_string(state, &id)));

                draw_item(state, ui, &id)
            });
        }
    });
}

lazy_static! {
    static ref INDEX_COUNTER: Mutex<usize> = Mutex::new(0);
}

pub fn reset_callback_counter() {
    *INDEX_COUNTER.lock().unwrap() = 0;
}

pub struct GameEguiCallback {
    instance: InstanceData,
    model: Id,
    index: usize,
}

impl GameEguiCallback {
    pub fn new(instance: InstanceData, model: Id, rect: Rect, screen_rect: Rect) -> Self {
        let mut counter = INDEX_COUNTER.lock().unwrap();

        let inside = screen_rect.intersect(rect);
        let sign = rect.center() - inside.center();

        let sx = rect.width() / inside.width();
        let sy = rect.height() / inside.height();

        let dx = (sx - 1.0) * sign.x.signum();
        let dy = (sy - 1.0) * sign.y.signum();

        let result = Self {
            instance: instance
                .add_world_matrix_left(Matrix4::from_translation(vec3(dx, dy, 0.0)))
                .add_world_matrix_right(Matrix4::from_scale(vec3(sx, sy, 1.0))),
            model,
            index: *counter,
        };
        *counter += 1;

        result
    }
}

impl CallbackTrait for GameEguiCallback {
    fn prepare(
        &self,
        _device: &Device,
        _queue: &Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<CommandBuffer> {
        let resource_man = callback_resources
            .get::<Arc<ResourceManager>>()
            .unwrap()
            .clone();
        let start_instant = *callback_resources.get::<Instant>().unwrap();
        let animation_map = callback_resources.get_mut::<AnimationMap>().unwrap();

        try_add_animation(&resource_man, start_instant, self.model, animation_map);

        callback_resources
            .entry::<Vec<(InstanceData, Id, usize)>>()
            .or_insert_with(Vec::new)
            .push((self.instance, self.model, self.index));

        Vec::new()
    }

    fn finish_prepare(
        &self,
        device: &Device,
        queue: &Queue,
        _egui_encoder: &mut CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<CommandBuffer> {
        if let Some(mut instances) = callback_resources.remove::<Vec<(InstanceData, Id, usize)>>() {
            instances.sort_by_key(|v| v.1);

            let resource_man = callback_resources
                .get::<Arc<ResourceManager>>()
                .unwrap()
                .clone();

            let animation_map = callback_resources.get::<AnimationMap>().unwrap();

            let (instances, draws, _count, matrix_data) =
                gpu::indirect_instance(&resource_man, &instances, false, animation_map);

            {
                let gui_resources = callback_resources.get_mut::<GuiResources>().unwrap();

                gpu::create_or_write_buffer(
                    device,
                    queue,
                    &mut gui_resources.instance_buffer,
                    bytemuck::cast_slice(instances.as_slice()),
                );

                queue.write_buffer(
                    &gui_resources.matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(matrix_data.as_slice()),
                );
            }

            callback_resources.insert(draws);
        }

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: PaintCallbackInfo,
        render_pass: &mut RenderPass<'a>,
        callback_resources: &'a CallbackResources,
    ) {
        if let Some(draws) =
            callback_resources.get::<HashMap<Id, Vec<(DrawIndexedIndirectArgs, usize)>>>()
        {
            let gui_resources = callback_resources.get::<GuiResources>().unwrap();
            let global_buffers = callback_resources.get::<Arc<GlobalBuffers>>().unwrap();

            render_pass.set_pipeline(&gui_resources.pipeline);
            render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
            render_pass.set_vertex_buffer(0, global_buffers.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, gui_resources.instance_buffer.slice(..));
            render_pass
                .set_index_buffer(global_buffers.index_buffer.slice(..), IndexFormat::Uint16);

            for (draw, ..) in draws[&self.model].iter().filter(|v| v.1 == self.index) {
                render_pass.draw_indexed(
                    draw.first_index..(draw.first_index + draw.index_count),
                    draw.base_vertex,
                    draw.first_instance..(draw.first_instance + draw.instance_count),
                );
            }
        }
    }
}

pub fn render_ui(
    state: &mut GameState,
    result: &mut anyhow::Result<bool>,
    target: &EventLoopWindowTarget<()>,
) {
    if state.input_handler.key_active(KeyActions::Debug) {
        #[cfg(debug_assertions)]
        state.gui.context.set_debug_on_hover(true);

        debug::debugger(state);
    } else {
        #[cfg(debug_assertions)]
        state.gui.context.set_debug_on_hover(false);
    }

    if state.gui_state.popup == PopupState::None {
        match state.gui_state.screen {
            Screen::Ingame => {
                if !state.input_handler.key_active(KeyActions::HideGui) {
                    if let Some(map_info) = state.loop_store.map_info.as_ref().map(|v| v.0.clone())
                    {
                        let mut lock = map_info.blocking_lock();
                        let game_data = &mut lock.data;

                        if state.input_handler.key_active(KeyActions::Player) {
                            player::player(state, game_data);
                        }

                        // tile_info
                        info::info(state);

                        // tile_config
                        tile_config::tile_config(state, game_data);

                        let (selection_send, selection_recv) = oneshot::channel();

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
                        state.gui.context.layer_painter(LayerId::background()).add(
                            egui_wgpu::Callback::new_paint_callback(
                                state.gui.context.screen_rect(),
                                GameEguiCallback::new(
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
                                    state.gui.context.screen_rect(),
                                    state.gui.context.screen_rect(),
                                ),
                            ),
                        );
                    }

                    if let Some(coord) = state.gui_state.linking_tile {
                        state.renderer.extra_instances.push((
                            InstanceData::default()
                                .with_color_offset(colors::RED.to_array())
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
                                .get(&state.resource_man.registry.data_ids.not_targeted)
                                .cloned()
                                .and_then(Data::into_bool)
                                .unwrap_or(false)
                        {
                            state.renderer.extra_instances.push((
                                InstanceData::default()
                                    .with_color_offset(colors::RED.to_array())
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

                if state.gui_state.research_open {
                    research::research_ui(state);
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

    state
        .renderer
        .tile_tints
        .insert(state.camera.pointing_at, colors::RED.with_alpha(0.2));

    for coord in &state.gui_state.grouped_tiles {
        state
            .renderer
            .tile_tints
            .insert(*coord, colors::ORANGE.with_alpha(0.4));
    }

    if state.input_handler.control_held {
        if let Some(start) = state.gui_state.initial_cursor_position {
            let direction = state.camera.pointing_at - start;

            if start != state.camera.pointing_at {
                state.renderer.extra_instances.push((
                    InstanceData::default()
                        .with_color_offset(colors::LIGHT_BLUE.to_array())
                        .with_light_pos(state.camera.get_pos().as_vec3(), None)
                        .with_world_matrix(state.camera.get_matrix().as_mat4())
                        .with_model_matrix(make_line(
                            HEX_GRID_LAYOUT.hex_to_world_pos(*start),
                            HEX_GRID_LAYOUT.hex_to_world_pos(*state.camera.pointing_at),
                        )),
                    state.resource_man.registry.model_ids.cube1x1,
                ));
            }

            for coord in &state.gui_state.grouped_tiles {
                let dest = *coord + direction;
                state
                    .renderer
                    .tile_tints
                    .insert(dest, colors::LIGHT_BLUE.with_alpha(0.3));
            }
        }
    }

    error::error_popup(state);
}
