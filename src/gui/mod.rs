use enum_map::{enum_map, Enum, EnumMap};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use hashbrown::{HashMap, HashSet};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::{cell::Cell, fmt::Debug, time::Instant};
use std::{collections::BTreeMap, mem};
use tokio::sync::oneshot;
use wgpu::IndexFormat;
use wgpu::{util::DrawIndexedIndirectArgs, Device, Queue};
use winit::{event_loop::EventLoopWindowTarget, window::Window};
use yakui_wgpu::{CallbackTrait, YakuiWgpu};
use yakui_winit::YakuiWinit;

use automancy_defs::glam::{dvec2, dvec3, vec3};
use automancy_defs::id::Id;
use automancy_defs::math::Vec2;
use automancy_defs::math::{Float, Matrix4, FAR, HEX_GRID_LAYOUT};
use automancy_defs::rendering::{make_line, InstanceData};
use automancy_defs::{bytemuck, colors, math, window};
use automancy_defs::{coord::TileCoord, glam::vec2};
use automancy_resources::data::item::Item;
use automancy_resources::data::Data;
use automancy_resources::ResourceManager;
use yakui::{
    column, constrained,
    font::{Font, Fonts},
    offset,
    paint::PaintCall,
    row,
    util::widget,
    widget::Widget,
    widgets::{Absolute, Layer},
    Alignment, Constraints, Dim2, Pivot, Rect, Response, Yakui,
};

use crate::game::TAKE_ITEM_ANIMATION_SPEED;
use crate::gpu::{AnimationMap, GlobalBuffers, GuiResources};
use crate::input::KeyActions;
use crate::renderer::try_add_animation;
use crate::{gpu, GameState};

use self::components::{
    hover::hover_tip,
    interactive::interactive,
    scrollable::scroll_vertical,
    select::radio,
    text::{label_text, symbol_text, Text},
    textbox::textbox,
};

pub mod components;

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
    /// the stored initial cursor position, for moving tiles
    pub initial_cursor_position: Option<TileCoord>,
    /// the current tile placement target direction, only Some when shift is held
    /// TODO shift is only on keyboard
    pub placement_direction: Option<TileCoord>,
    pub prev_placement_direction: Option<TileCoord>,

    pub tile_config_ui_position: Vec2,

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
            initial_cursor_position: Default::default(),
            placement_direction: Default::default(),
            prev_placement_direction: Default::default(),

            tile_config_ui_position: vec2(0.1, 0.1),

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

thread_local! {
    static HOVER_TIP: Cell<Option<Text>> = Cell::default();
}

fn render_info_tip(state: &mut GameState) {
    if let Some(tip) = HOVER_TIP.take() {
        Layer::new().show(|| {
            hover_tip(|| {
                constrained(
                    Constraints::loose(state.gui.yak.layout_dom().viewport().size()),
                    || {
                        tip.show();
                    },
                );
            });
        });
    }
}

pub fn info_tip(info: &str) {
    let label = interactive(|| {
        symbol_text("\u{f449}", colors::BLACK).show();
    });

    if label.hovering {
        HOVER_TIP.set(Some(label_text(info)));
    }
}

fn take_item_animation(state: &mut GameState, item: Item, dst_rect: Rect) {
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

            let pos = src_rect.pos().lerp(dst_rect.pos(), d);
            let size = src_rect.size().lerp(dst_rect.size(), d);

            Layer::new().show(|| {
                offset(pos, || {
                    ui_game_object(
                        InstanceData::default()
                            .with_world_matrix(math::view(dvec3(0.0, 0.0, 1.0)).as_mat4()),
                        state.resource_man.get_item_model(item.model),
                        size,
                    );
                });
            });
        }
    }
}

/// Draws a search bar.
pub fn searchable_id(
    ids: &[Id],
    new_id: &mut Option<Id>,
    field: TextField,
    hint_text: String,
    to_string: &'static impl Fn(&GameState, &Id) -> String,
    draw_item: &'static impl Fn(&mut GameState, &Id),
    state: &mut GameState,
) {
    textbox(state.gui_state.text_field.get(field), &hint_text);

    scroll_vertical(200.0, || {
        column(|| {
            let ids = if !state.gui_state.text_field.get(field).is_empty() {
                let text = state.gui_state.text_field.get(field).clone();
                let mut filtered = ids
                    .iter()
                    .flat_map(|id| {
                        let score = state
                            .gui_state
                            .text_field
                            .fuse
                            .fuzzy_match(&to_string(state, id), &text);

                        if score.unwrap_or(0) <= 5 {
                            None
                        } else {
                            Some(*id).zip(score)
                        }
                    })
                    .collect::<Vec<_>>();

                filtered.sort_unstable_by(|a, b| a.1.cmp(&b.1));

                filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
            } else {
                ids.to_vec()
            };

            for id in ids {
                row(|| {
                    radio(new_id, Some(id), || {
                        draw_item(state, &id);
                    });
                });
            }
        });
    });
}

pub type YakuiRenderResources = (
    Arc<ResourceManager>,
    Arc<GlobalBuffers>,
    Option<GuiResources>,
    AnimationMap,
    Option<Vec<(InstanceData, Id, usize)>>,
    HashMap<Id, Vec<(DrawIndexedIndirectArgs, usize)>>,
);

thread_local! {
    static START_INSTANT: Cell<Option<Instant>> = const { Cell::new(None) };
    static INDEX_COUNTER: Cell<usize> = const { Cell::new(0) };
}

pub fn init_custom_paint_state(start_instant: Instant) {
    START_INSTANT.set(Some(start_instant));
}

pub fn reset_custom_paint_state() {
    INDEX_COUNTER.replace(0);
}

#[derive(Debug, Clone, Copy)]
pub struct GameElement {
    instance: InstanceData,
    model: Id,
    index: usize,
    size: Vec2,
}

pub fn ui_game_object(instance: InstanceData, model: Id, size: Vec2) -> Response<Option<Rect>> {
    GameElement::new(instance, model, size).show()
}

impl GameElement {
    pub fn new(instance: InstanceData, model: Id, size: Vec2) -> Self {
        let index = INDEX_COUNTER.get();

        let result = Self {
            instance,
            model,
            index,
            size,
        };
        INDEX_COUNTER.set(index + 1);

        result
    }

    pub fn show(self) -> Response<Option<Rect>> {
        widget::<GameElementWidget>(Some(self))
    }
}

#[derive(Debug, Clone)]
pub struct GameElementWidget {
    paint: Cell<Option<GameElement>>,
    layout_rect: Cell<Option<Rect>>,
    clip: Cell<Rect>,
    adjusted_matrix: Cell<Option<Matrix4>>,
}

impl CallbackTrait<YakuiRenderResources> for GameElementWidget {
    fn prepare(
        &self,
        (
        resource_man,
        _global_buffers,
        _gui_resources,
        animation_map,
        instances,
        _draws,
    ): &mut YakuiRenderResources,
    ) {
        if let Some(mut paint) = self.paint.get() {
            let start_instant = START_INSTANT.get().unwrap();
            try_add_animation(resource_man, start_instant, paint.model, animation_map);

            if let Some(m) = self.adjusted_matrix.get() {
                paint.instance = paint.instance.with_world_matrix(m);
            }

            instances
                .as_mut()
                .unwrap()
                .push((paint.instance, paint.model, paint.index));
        }
    }

    fn finish_prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (
            resource_man,
            _global_buffers,
            gui_resources,
            animation_map,
            instances,
            draws,
        ): &mut YakuiRenderResources,
    ) {
        if let Some(mut instances) = instances.take() {
            let gui_resources = gui_resources.as_mut().unwrap();

            instances.sort_by_key(|v| v.1);

            let (instances, draws_result, _count, matrix_data) =
                gpu::indirect_instance(resource_man, &instances, false, animation_map);

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

            *draws = draws_result;
        }
    }

    fn paint<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        (
            _resource_man,
            global_buffers,
            gui_resources,
            _animation_map,
            _instances,
            draws,
        ): &'a YakuiRenderResources,
    ) {
        let gui_resources = gui_resources.as_ref().unwrap();

        render_pass.set_pipeline(&gui_resources.pipeline);
        render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
        render_pass.set_vertex_buffer(0, global_buffers.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, gui_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(global_buffers.index_buffer.slice(..), IndexFormat::Uint16);

        let clip = self.clip.get();

        if clip.size().x > 0.0 && clip.size().y > 0.0 && clip.pos().x >= 0.0 && clip.pos().y >= 0.0
        {
            render_pass.set_viewport(
                clip.pos().x,
                clip.pos().y,
                clip.size().x,
                clip.size().y,
                0.0,
                1.0,
            );
        }

        for (draw, ..) in draws[&self.paint.get().unwrap().model]
            .iter()
            .filter(|v| v.1 == self.paint.get().unwrap().index)
        {
            render_pass.draw_indexed(
                draw.first_index..(draw.first_index + draw.index_count),
                draw.base_vertex,
                draw.first_instance..(draw.first_instance + draw.instance_count),
            );
        }

        {
            render_pass.set_pipeline(&gui_resources.depth_clear_pipeline);
            render_pass.draw(0..3, 0..1);
        }
    }
}

impl Widget for GameElementWidget {
    type Props<'a> = Option<GameElement>;
    type Response = Option<Rect>;

    fn new() -> Self {
        Self {
            paint: Cell::default(),
            layout_rect: Cell::default(),
            clip: Cell::new(Rect::ZERO),
            adjusted_matrix: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.paint.set(props);

        self.layout_rect.get()
    }

    fn layout(
        &self,
        ctx: yakui::widget::LayoutContext<'_>,
        _constraints: yakui::Constraints,
    ) -> yakui::Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        if let Some(layout_node) = ctx.layout.get(ctx.dom.current()) {
            self.layout_rect.set(Some(layout_node.rect));
        }

        if let Some(paint) = self.paint.get() {
            paint.size
        } else {
            Vec2::ZERO
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let clip = ctx.paint.get_current_clip();

        if let Some((paint, layout_rect)) = self.paint.get().zip(self.layout_rect.get()) {
            let clip = self.clip.get();

            if clip.size().x > 0.0 && clip.size().y > 0.0 {
                let rect = layout_rect;

                let inside = clip.constrain(layout_rect);

                let sign = (rect.max() - rect.size() / 2.0) - (inside.max() - inside.size() / 2.0);

                let sx = rect.size().x / inside.size().x;
                let sy = rect.size().y / inside.size().y;

                let dx = (sx - 1.0) * sign.x.signum();
                let dy = (sy - 1.0) * sign.y.signum();

                self.adjusted_matrix.set(Some(
                    Matrix4::from_translation(vec3(dx, dy, 0.0))
                        * paint
                            .instance
                            .get_world_matrix()
                            .unwrap_or(Matrix4::IDENTITY)
                        * Matrix4::from_scale(vec3(sx, sy, 1.0)),
                ));
            }
        }

        if let Some(clip) = clip {
            self.clip.set(clip);
        }

        if let Some(layer) = ctx.paint.layers_mut().current_mut() {
            layer
                .calls
                .push((PaintCall::Custom(yakui_wgpu::cast(self.clone())), clip));
        }
    }
}

pub fn render_ui(
    state: &mut GameState,
    result: &mut anyhow::Result<bool>,
    target: &EventLoopWindowTarget<()>,
) {
    if state.gui_state.popup == PopupState::None {
        match state.gui_state.screen {
            Screen::Ingame => {
                if !state.input_handler.key_active(KeyActions::HideGui) {
                    if let Some(map_info) = state.loop_store.map_info.as_ref().map(|v| v.0.clone())
                    {
                        let mut lock = map_info.blocking_lock();
                        let game_data = &mut lock.data;

                        // tile_info
                        info::info_ui(state);

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

                        // tile_config
                        tile_config::tile_config_ui(state, game_data);

                        if state.input_handler.key_active(KeyActions::Player) {
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

    if state.input_handler.control_held {
        if let Some(start) = state.gui_state.initial_cursor_position {
            let direction = state.camera.pointing_at - start;

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

            for coord in &state.gui_state.grouped_tiles {
                let dest = *coord + direction;
                state
                    .renderer
                    .tile_tints
                    .insert(dest, colors::LIGHT_BLUE.with_alpha(0.3).to_linear());
            }
        }
    }

    if state.input_handler.key_active(KeyActions::Debug) {
        debug::debugger(state);
    }

    error::error_popup(state);
}
