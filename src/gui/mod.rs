use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{CursorIcon, PaintCallbackInfo, Rect, ScrollArea, TextEdit, Ui, Widget, WidgetText};
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use enum_map::{enum_map, Enum, EnumMap};
use fuse_rust::Fuse;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use wgpu::util::DrawIndexedIndirectArgs;
use wgpu::{CommandBuffer, CommandEncoder, Device, IndexFormat, Queue, RenderPass};

use automancy_defs::bytemuck;
use automancy_defs::glam::vec3;
use automancy_defs::id::Id;
use automancy_defs::math::{Float, Matrix4};
use automancy_defs::rendering::InstanceData;
use automancy_resources::ResourceManager;

use crate::gpu;
use crate::gpu::{AnimationMap, GlobalBuffers, GuiResources};
use crate::renderer::try_add_animation;

pub mod debug;
pub mod error;
pub mod info;
pub mod item;
pub mod menu;
pub mod player;
pub mod popup;
pub mod tile_config;
pub mod tile_selection;

pub const SMALL_ICON_SIZE: Float = 24.0;
pub const MEDIUM_ICON_SIZE: Float = 48.0;
pub const LARGE_ICON_SIZE: Float = 96.0;

pub struct GuiState {
    pub screen: Screen,
    pub substate: SubState,
    pub popup: PopupState,
    pub show_debugger: bool,
    pub previous: Option<Screen>,
    pub text_field: TextFieldState,
    pub tile_selection_category: Option<Id>,
}

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Screen {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
    Research,
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

impl Default for GuiState {
    fn default() -> Self {
        GuiState {
            screen: Screen::MainMenu,
            substate: SubState::None,
            popup: PopupState::None,
            show_debugger: false,
            previous: None,
            text_field: Default::default(),
            tile_selection_category: None,
        }
    }
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
        when: &'static dyn Fn(&mut GuiState) -> bool,
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

    /// Draws a search bar.
    pub fn searchable_id<'a>(
        &mut self,
        ui: &mut Ui,
        resource_man: &'a ResourceManager,
        ids: &[Id],
        new_id: &mut Option<Id>,
        field: TextField,
        hint_text: impl Into<WidgetText>,
        to_string: &'static impl Fn(&'a ResourceManager, &Id) -> &'a str,
        draw_item: &'static impl Fn(&mut Ui, &ResourceManager, &Id),
    ) {
        TextEdit::singleline(self.get(field))
            .hint_text(hint_text)
            .ui(ui);

        ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            ui.set_width(ui.available_width());

            let ids = if !self.get(field).is_empty() {
                let text = self.get(field).clone();
                let mut filtered = ids
                    .iter()
                    .flat_map(|id| {
                        let result = self
                            .fuse
                            .search_text_in_string(&text, to_string(resource_man, id));
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

                    ui.radio_value(
                        new_id,
                        Some(id),
                        format!("{}:", to_string(resource_man, &id)),
                    );

                    draw_item(ui, resource_man, &id)
                });
            }
        });
    }
}

pub fn info_hover(ui: &mut Ui, info: impl Into<WidgetText>) {
    ui.label("ⓘ")
        .on_hover_cursor(CursorIcon::Help)
        .on_hover_ui(|ui| {
            ui.label(info);
        });
}

lazy_static! {
    static ref INDEX_COUNTER: Mutex<u32> = Mutex::new(0);
}

pub fn reset_callback_counter() {
    *INDEX_COUNTER.lock().unwrap() = 0;
}

pub struct GameEguiCallback {
    instance: InstanceData,
    model: Id,
    index: u32,
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
            .entry::<Vec<(InstanceData, Id, u32)>>()
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
        if let Some(mut instances) = callback_resources.remove::<Vec<(InstanceData, Id, u32)>>() {
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
            callback_resources.get::<HashMap<Id, Vec<(DrawIndexedIndirectArgs, u32)>>>()
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
