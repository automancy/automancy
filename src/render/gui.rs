use std::f32::consts::FRAC_PI_4;
use std::fs::File;
use std::path::Path;

use std::sync::Arc;

use cgmath::{point2, point3, vec3, MetricSpace};
use egui::epaint::Shadow;
use egui::style::{default_text_styles, Margin, WidgetVisuals, Widgets};
use egui::FontFamily::{Monospace, Proportional};
use egui::{
    vec2, Align, Align2, Color32, CursorIcon, DragValue, FontData, FontDefinitions, FontId, Frame,
    PaintCallback, Rgba, RichText, Rounding, ScrollArea, Sense, Stroke, Style, TextStyle,
    TopBottomPanel, Ui, Vec2, Visuals, WidgetText, Window,
};
use egui_winit_vulkano::{CallbackFn, Gui, GuiConfig};
use fuse_rust::Fuse;
use futures::channel::mpsc;
use genmesh::{EmitTriangles, Quad};
use hashbrown::HashMap;
use hexagon_tiles::traits::HexDirection;

use ractor::ActorRef;
use rune::Any;
use tokio::runtime::Runtime;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::SampleCount::Sample4;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};

use winit::event_loop::{ControlFlow, EventLoop};

use crate::game::run::error::{error_to_key, error_to_string};
use crate::game::run::event::{shutdown_graceful, EventLoopStorage};
use crate::game::run::setup::GameSetup;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::coord::TileHex;
use crate::game::tile::entity::{Data, TileEntityMsg, TileModifier};
use crate::game::GameMsg;
use crate::render::camera::hex_to_normalized;
use crate::render::data::{GameUBO, GameVertex, InstanceData};
use crate::render::gpu::Gpu;
use crate::render::renderer::Renderer;
use crate::render::{gpu, gui};
use crate::resource::tile::TileType;
use crate::resource::ResourceManager;
use crate::util::cg::{perspective, DPoint2, DPoint3, Double, Float, Matrix4, Vector3};
use crate::util::colors;
use crate::util::id::{id_static, Id, Interner};
use crate::IOSEVKA_FONT;

#[derive(Clone, Copy, Any)]
pub struct GuiIds {
    #[rune(get, copy)]
    pub tile_config: Id,
    #[rune(get, copy)]
    pub tile_info: Id,
    #[rune(get, copy)]
    pub tile_config_script: Id,
    #[rune(get, copy)]
    pub tile_config_storage: Id,
    #[rune(get, copy)]
    pub tile_config_target: Id,
    #[rune(get, copy)]
    pub error_popup: Id,
    #[rune(get, copy)]
    pub debug_menu: Id,

    #[rune(get, copy)]
    pub lbl_amount: Id,
    #[rune(get, copy)]
    pub lbl_link_destination: Id,

    #[rune(get, copy)]
    pub btn_confirm: Id,
    #[rune(get, copy)]
    pub btn_exit: Id,
    #[rune(get, copy)]
    pub btn_cancel: Id,
    #[rune(get, copy)]
    pub btn_link_network: Id,
}

impl GuiIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            tile_config: id_static("automancy", "tile_config").to_id(interner),
            tile_info: id_static("automancy", "tile_info").to_id(interner),
            tile_config_script: id_static("automancy", "tile_config_script").to_id(interner),
            tile_config_storage: id_static("automancy", "tile_config_storage").to_id(interner),
            tile_config_target: id_static("automancy", "tile_config_target").to_id(interner),
            error_popup: id_static("automancy", "error_popup").to_id(interner),
            debug_menu: id_static("automancy", "debug_menu").to_id(interner),

            lbl_amount: id_static("automancy", "error_popup").to_id(interner),
            lbl_link_destination: id_static("automancy", "lbl_link_destination").to_id(interner),

            btn_confirm: id_static("automancy", "btn_confirm").to_id(interner),
            btn_exit: id_static("automancy", "btn_exit").to_id(interner),
            btn_cancel: id_static("automancy", "btn_cancel").to_id(interner),
            btn_link_network: id_static("automancy", "btn_link_network").to_id(interner),
        }
    }
}

pub enum GuiState {
    Main,
    MapLoad,
    Options,
    Ingame,
}
fn init_fonts(gui: &Gui) {
    let mut fonts = FontDefinitions::default();
    let iosevka = "iosevka";

    fonts
        .font_data
        .insert(iosevka.to_owned(), FontData::from_static(IOSEVKA_FONT));

    fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, iosevka.to_owned());
    fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, iosevka.to_owned());

    gui.context().set_fonts(fonts);
}

fn init_styles(gui: &Gui) {
    gui.context().set_style(Style {
        override_text_style: None,
        override_font_id: None,
        text_styles: [
            (TextStyle::Small, FontId::new(9.0, Proportional)),
            (TextStyle::Body, FontId::new(13.0, Proportional)),
            (TextStyle::Button, FontId::new(13.0, Proportional)),
            (TextStyle::Heading, FontId::new(19.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, Monospace)),
        ]
        .into(),
        wrap: None,
        visuals: Visuals {
            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(248),
                    bg_fill: Color32::from_gray(170),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)), // separators, indentation lines
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(200), // button background
                    bg_fill: Color32::from_gray(200),      // checkbox background
                    bg_stroke: Default::default(),
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(190),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 1.0,
                },
                active: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(165),
                    bg_fill: Color32::from_gray(180),
                    bg_stroke: Stroke::new(1.0, Color32::BLACK),
                    fg_stroke: Stroke::new(2.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 1.0,
                },
                open: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(210),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                    fg_stroke: Stroke::new(1.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
            },
            ..Visuals::light()
        },
        ..Default::default()
    });
}

pub fn init_gui(event_loop: &EventLoop<()>, gpu: &Gpu) -> Gui {
    let gui = Gui::new_with_subpass(
        event_loop,
        gpu.surface.clone(),
        gpu.queue.clone(),
        gpu.gui_subpass.clone(),
        GuiConfig {
            preferred_format: Some(gpu.alloc.swapchain.image_format()),
            is_overlay: true,
            samples: Sample4,
        },
    );

    init_fonts(&gui);
    init_styles(&gui);

    gui
}

pub fn default_frame() -> Frame {
    Frame::none()
        .fill(colors::WHITE.multiply(0.65).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: colors::DARK_GRAY.multiply(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
}

fn tile_paint(
    setup: &GameSetup,
    renderer: &Renderer,
    ui: &mut Ui,
    size: f32,
    id: Id,
    model: Id,
    selection_send: &mut mpsc::Sender<Id>,
) -> PaintCallback {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());

    response
        .clone()
        .on_hover_text(setup.resource_man.tile_name(&id));
    response.clone().on_hover_cursor(CursorIcon::Grab);

    let hover = if response.hovered() {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 1.0, 0.3)
    } else {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 0.0, 0.3)
    };
    if response.clicked() {
        selection_send.try_send(id).unwrap();
    }

    let pos = point3(0.0, 1.0 * hover + 0.5, 3.0 - 0.5 * hover);
    let matrix = perspective(FRAC_PI_4, 1.0, 0.01, 10.0)
        * Matrix4::look_to_rh(pos, vec3(0.0, 0.5 * hover + 0.2, 1.0), Vector3::unit_y());

    let pipeline = renderer.gpu.gui_pipeline.clone();
    let vertex_buffer = renderer.gpu.alloc.vertex_buffer.clone();
    let index_buffer = renderer.gpu.alloc.index_buffer.clone();
    let ubo_layout = pipeline.layout().set_layouts()[0].clone();
    let resource_man = setup.resource_man.clone();

    PaintCallback {
        rect,
        callback: Arc::new(CallbackFn::new(move |_info, context| {
            let ubo = GameUBO::new(matrix, pos);

            let uniform_buffer = Buffer::from_data(
                &context.resources.memory_allocator,
                BufferCreateInfo {
                    usage: BufferUsage::UNIFORM_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                ubo,
            )
            .unwrap();

            let ubo_set = PersistentDescriptorSet::new(
                context.resources.descriptor_set_allocator,
                ubo_layout.clone(),
                [WriteDescriptorSet::buffer(0, uniform_buffer)],
            )
            .unwrap();

            let instance = (InstanceData::default().into(), model);

            if let Some((indirect_commands, instance_buffer)) = gpu::indirect_instance(
                &context.resources.memory_allocator,
                &resource_man,
                &[instance],
            ) {
                context
                    .builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer))
                    .bind_index_buffer(index_buffer.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        ubo_set,
                    )
                    .draw_indexed_indirect(indirect_commands)
                    .unwrap();
            }
        })),
    }
}

fn paint_tile_selection(
    setup: &GameSetup,
    renderer: &Renderer,
    ui: &mut Ui,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    mut selection_send: mpsc::Sender<Id>,
) {
    let size = ui.available_height();

    setup
        .resource_man
        .ordered_tiles
        .iter()
        .flat_map(|id| {
            let resource = &setup.resource_man.registry.get_tile(id).unwrap();

            if resource.tile_type == TileType::Model {
                return None;
            }

            resource
                .models
                .get(*selected_tile_modifiers.get(id).unwrap_or(&0) as usize)
                .map(|v| (*id, *v))
        })
        .for_each(|(id, faces_index)| {
            let callback = tile_paint(
                setup,
                renderer,
                ui,
                size,
                id,
                faces_index,
                &mut selection_send,
            );

            ui.painter().add(callback);
        });
}

pub fn tile_selections(
    setup: &GameSetup,
    renderer: &Renderer,
    gui: &Gui,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    selection_send: mpsc::Sender<Id>,
) {
    TopBottomPanel::bottom("tile_selections")
        .show_separator_line(false)
        .resizable(false)
        .frame(default_frame().outer_margin(Margin::same(10.0)))
        .show(&gui.context(), |ui| {
            let spacing = ui.spacing_mut();

            spacing.interact_size.y = 70.0;
            spacing.scroll_bar_width = 0.0;
            spacing.scroll_bar_outer_margin = 0.0;

            ScrollArea::horizontal()
                .always_show_scroll(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        paint_tile_selection(
                            setup,
                            renderer,
                            ui,
                            selected_tile_modifiers,
                            selection_send,
                        );
                    });
                });
        });
}

pub fn tile_info(
    runtime: &Runtime,
    setup: &GameSetup,
    gui: &Gui,
    game: &ActorRef<GameMsg>,
    pointing_at: TileCoord,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_info]
            .to_string(),
    )
    .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.colored_label(colors::DARK_GRAY, pointing_at.to_string());

        let tile_entity = runtime
            .block_on(game.call(|reply| GameMsg::GetTileEntity(pointing_at, reply), None))
            .unwrap()
            .unwrap();

        let tile = runtime
            .block_on(game.call(|reply| GameMsg::GetTile(pointing_at, reply), None))
            .unwrap()
            .unwrap();

        if let Some((tile_entity, (id, _))) = tile_entity.zip(tile) {
            ui.label(setup.resource_man.tile_name(&id));

            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            if let Some(inventory) = data.get("buffer").and_then(Data::as_inventory) {
                for (id, amount) in inventory.0.iter() {
                    ui.label(format!("{} - {}", setup.resource_man.item_name(id), amount));
                }
            }
            //ui.label(format!("State: {}", ask(sys, &game, )))
        }
    });
}

pub fn error_popup(
    setup: &mut GameSetup,
    gui: &mut Gui,
    runtime: &Runtime,
    loop_store: &mut EventLoopStorage,
    control_flow: &mut ControlFlow,
) {
    let error = setup.resource_man.error_man.peek().unwrap();
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.error_popup]
            .to_string(),
    )
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.label(format!("ID: {}", error_to_key(&error, &setup.resource_man)));
        ui.label(error_to_string(&error, &setup.resource_man));
        //FIXME why are the buttons not right aligned
        ui.with_layout(ui.layout().with_main_align(Align::RIGHT), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_confirm]
                            .to_string(),
                    )
                    .clicked()
                {
                    setup.resource_man.error_man.pop();
                }
                if ui
                    .button(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_exit]
                            .to_string(),
                    )
                    .clicked()
                {
                    shutdown_graceful(setup, runtime, loop_store, control_flow)
                        .expect("Failed to shut down gracefully!");
                }
            });
        });
    });
}

pub fn debugger(
    setup: &GameSetup,
    gui: &mut Gui,
    runtime: &Runtime,
    game: &ActorRef<GameMsg>,
    renderer: &Renderer,
    loop_store: &mut EventLoopStorage,
) {
    let resource_man = setup.resource_man.clone();
    let device_name = renderer
        .gpu
        .alloc
        .physical_device
        .properties()
        .device_name
        .clone();
    let fps = 1.0 / loop_store.elapsed.as_secs_f64();
    let api_version = renderer
        .gpu
        .surface
        .instance()
        .max_api_version()
        .to_string();
    let tiles = resource_man.ordered_tiles.len();
    let reg_tiles = resource_man.registry.tiles.len();
    let items = resource_man.ordered_items.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.meshes.len();
    let models = resource_man.raw_models.len();

    let map = runtime
        .block_on(game.call(GameMsg::GetMapInfo, Some(loop_store.elapsed)))
        .unwrap()
        .unwrap();
    let map_name = map.map_name;
    let data_size = map.data;
    let tile_count = map.tiles;
    let maps = &resource_man.maps;
    let map_file = &maps.get(&format!("{map_name}.bin"));
    let file_size = if map_file.is_some() {
        map_file.unwrap().len()
    } else {
        0
    };
    Window::new(
        setup.resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu]
            .to_string(),
    )
    .resizable(false)
    .default_width(600.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.label(format!("FPS: {fps:.1}"));
        ui.label(format!("Device: {device_name} API {api_version}"));
        ui.label(format!(
            "ResourceMan: {tiles}/{reg_tiles}T {items}/{reg_items}I {functions}F {tags}Ta {scripts}S {audio}A {meshes}/{models}M"
        ));
        ui.label(format!(
            "Map \"{map_name}\" ({map_name}.bin): {data_size}D {tile_count}T, {file_size}B (on open)"
        ))
    });
}
pub fn main_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    runtime: &Runtime,
    loop_store: &mut EventLoopStorage,
    control_flow: &mut ControlFlow,
) {
    Window::new("main_menu".to_string())
        .resizable(false)
        .default_width(175.0)
        .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
        .frame(default_frame().inner_margin(10.0))
        .movable(false)
        .title_bar(false)
        .show(&gui.context(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    ui.label(RichText::new("automancy").size(30.0));
                    if ui.button(RichText::new("enter game").heading()).clicked() {
                        setup.gui_state = GuiState::MapLoad
                    };
                    if ui.button(RichText::new("options").heading()).clicked() {
                        setup.gui_state = GuiState::Options
                    };
                    if ui.button(RichText::new("fedi").heading()).clicked() {
                        webbrowser::open("https://gamedev.lgbt/@automancy")
                            .expect("Failed to open web browser");
                    };
                    if ui.button(RichText::new("source").heading()).clicked() {
                        webbrowser::open("https://github.com/sorcerers-class/automancy")
                            .expect("Failed to open web browser");
                    };
                    if ui.button(RichText::new("quit").heading()).clicked() {
                        shutdown_graceful(setup, runtime, loop_store, control_flow)
                            .expect("Failed to shutdown gracefully!");
                    };
                    ui.label("v0.1.0")
                },
            );
        });
}
pub fn options_menu(setup: &mut GameSetup, gui: &mut Gui) {
    Window::new("Options".to_string())
        .resizable(false)
        .default_width(175.0)
        .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
        .frame(default_frame().inner_margin(10.0))
        .show(&gui.context(), |ui| {
            ui.label("Not yet implemented");
            if ui.button("Ok").clicked() {
                setup.gui_state = GuiState::Main
            }
        });
}
pub fn add_direction(ui: &mut Ui, target_coord: &mut Option<TileCoord>, n: usize) {
    let coord = TileHex::NEIGHBORS[(n + 2) % 6];
    let coord = Some(coord.into());

    ui.selectable_value(
        target_coord,
        coord,
        match n {
            0 => "↗",
            1 => "➡",
            2 => "↘",
            3 => "↙",
            4 => "⬅",
            5 => "↖",
            _ => "",
        },
    );
}

pub fn searchable_id<'a>(
    ui: &mut Ui,
    resource_man: &'a ResourceManager,
    fuse: &Fuse,
    ids: &[Id],
    new_id: &mut Option<Id>,
    filter: &mut String,
    name: &'static impl Fn(&'a ResourceManager, &Id) -> &'a str,
) {
    ui.text_edit_singleline(filter);

    ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
        ui.set_width(ui.available_width());

        let ids = if !filter.is_empty() {
            let mut filtered = ids
                .iter()
                .flat_map(|id| {
                    let result = fuse.search_text_in_string(filter, name(resource_man, id));
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

        ids.iter().for_each(|script| {
            ui.radio_value(new_id, Some(*script), name(resource_man, script));
        })
    });
}

pub fn targets(ui: &mut Ui, new_target_coord: &mut Option<TileCoord>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            add_direction(ui, new_target_coord, 5);
            add_direction(ui, new_target_coord, 0);
        });

        ui.horizontal(|ui| {
            add_direction(ui, new_target_coord, 4);
            ui.selectable_value(new_target_coord, None, "❌");
            add_direction(ui, new_target_coord, 1);
        });

        ui.horizontal(|ui| {
            ui.add_space(15.0);
            add_direction(ui, new_target_coord, 3);
            add_direction(ui, new_target_coord, 2);
        });
    });
}

pub fn tile_config(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui: &Gui,
    game: &ActorRef<GameMsg>,
    extra_vertices: &mut Vec<GameVertex>,
) {
    let window_size = setup.window.inner_size();

    if let Some(config_open) = loop_store.config_open {
        let tile = runtime
            .block_on(game.call(|reply| GameMsg::GetTile(config_open, reply), None))
            .unwrap()
            .unwrap();

        let tile_entity = runtime
            .block_on(game.call(|reply| GameMsg::GetTileEntity(config_open, reply), None))
            .unwrap()
            .unwrap();

        if let Some(((id, _), tile_entity)) = tile.zip(tile_entity) {
            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            let current_amount = data
                .get("amount")
                .and_then(Data::as_amount)
                .cloned()
                .unwrap_or(0);
            let mut new_amount = current_amount;

            let current_script = data.get("script").and_then(Data::as_id).cloned();
            let mut new_script = current_script;

            let current_storage = data.get("storage").and_then(Data::as_id).cloned();
            let mut new_storage = current_storage;

            let current_target_coord = data.get("target").and_then(Data::as_coord).cloned();
            let mut new_target_coord = current_target_coord;

            // tile_config
            Window::new(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_config]
                    .to_string(),
            )
            .resizable(false)
            .auto_sized()
            .constrain(true)
            .frame(setup.frame.inner_margin(Margin::same(10.0)))
            .show(&gui.context(), |ui| {
                const MARGIN: Float = 8.0;

                ui.set_max_width(300.0);

                match &setup.resource_man.registry.get_tile(&id).unwrap().tile_type {
                    TileType::Machine(scripts) => {
                        let script_text = if let Some(script) = new_script
                            .as_ref()
                            .and_then(|id| setup.resource_man.registry.get_script(id))
                        {
                            let input = if let Some(inputs) = script.instructions.inputs {
                                inputs
                                    .iter()
                                    .map(|item_stack| {
                                        format!(
                                            " + {} ({})",
                                            setup.resource_man.item_name(&item_stack.item.id),
                                            item_stack.amount
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            } else {
                                String::new()
                            };

                            let output = if let Some(output) = script.instructions.output {
                                format!(
                                    "=> {} ({})",
                                    setup.resource_man.item_name(&output.item.id),
                                    output.amount
                                )
                            } else {
                                String::new()
                            };

                            if !input.is_empty() && !output.is_empty() {
                                format!("{input}\n{output}")
                            } else {
                                format!("{input}{output}")
                            }
                        } else {
                            setup.resource_man.translates.none.to_string()
                        };

                        ui.add_space(MARGIN);

                        ui.label(
                            setup.resource_man.translates.gui
                                [&setup.resource_man.registry.gui_ids.tile_config_script]
                                .as_str(),
                        );
                        ui.label(script_text);

                        ui.add_space(MARGIN);

                        searchable_id(
                            ui,
                            &setup.resource_man,
                            &loop_store.fuse,
                            scripts.as_slice(),
                            &mut new_script,
                            &mut loop_store.filter,
                            &ResourceManager::script_name,
                        );
                    }
                    TileType::Storage(storage) => {
                        let storage_text = if let Some(item) = new_storage
                            .as_ref()
                            .and_then(|id| setup.resource_man.registry.get_item(id))
                        {
                            setup.resource_man.item_name(&item.id).to_string()
                        } else {
                            setup.resource_man.translates.none.to_string()
                        };

                        let items = setup
                            .resource_man
                            .get_items(storage.id, &mut loop_store.tag_cache)
                            .iter()
                            .map(|item| item.id)
                            .collect::<Vec<_>>();

                        ui.add_space(MARGIN);

                        ui.label(
                            setup.resource_man.translates.gui
                                [&setup.resource_man.registry.gui_ids.tile_config_storage]
                                .as_str(),
                        );
                        ui.horizontal(|ui| {
                            ui.label(storage_text);
                            ui.add(
                                DragValue::new(&mut new_amount)
                                    .clamp_range(0..=65535)
                                    .speed(1.0)
                                    .prefix(
                                        setup.resource_man.translates.gui
                                            [&setup.resource_man.registry.gui_ids.lbl_amount]
                                            .to_string(),
                                    ),
                            );
                        });

                        ui.add_space(MARGIN);

                        searchable_id(
                            ui,
                            &setup.resource_man,
                            &loop_store.fuse,
                            items.as_slice(),
                            &mut new_storage,
                            &mut loop_store.filter,
                            &ResourceManager::item_name,
                        );
                    }
                    TileType::Transfer(id) => {
                        if id == &setup.resource_man.registry.tile_ids.master_node {
                            ui.add_space(MARGIN);

                            if ui
                                .button(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_link_network]
                                        .to_string(),
                                )
                                .clicked()
                            {
                                loop_store.linking_tile = Some(config_open);
                            };
                            ui.label(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.lbl_link_destination]
                                    .to_string(),
                            );

                            ui.add_space(MARGIN);
                        }

                        if id == &setup.resource_man.registry.tile_ids.node {
                            if let Some(tile_entity) =
                                runtime
                                    .block_on(game.call(
                                        |reply| GameMsg::GetTileEntity(config_open, reply),
                                        None,
                                    ))
                                    .unwrap()
                                    .unwrap()
                            {
                                let result = runtime
                                    .block_on(tile_entity.call(
                                        |reply| TileEntityMsg::GetDataValue("link", reply),
                                        None,
                                    ))
                                    .unwrap()
                                    .unwrap();

                                if let Some(link) = result.as_ref().and_then(Data::as_coord) {
                                    let DPoint3 { x, y, .. } = hex_to_normalized(
                                        window_size.width as Double,
                                        window_size.height as Double,
                                        setup.camera.get_pos(),
                                        config_open,
                                    );
                                    let a = point2(x, y);

                                    let DPoint3 { x, y, .. } = hex_to_normalized(
                                        window_size.width as Double,
                                        window_size.height as Double,
                                        setup.camera.get_pos(),
                                        config_open + *link,
                                    );
                                    let b = point2(x, y);

                                    extra_vertices.append(&mut gui::line(a, b, colors::RED));
                                }
                            }
                        }
                    }
                    _ => {}
                }

                if setup.resource_man.registry.get_tile(&id).unwrap().targeted {
                    ui.add_space(MARGIN);

                    ui.label(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.tile_config_target]
                            .as_str(),
                    );
                    targets(ui, &mut new_target_coord);
                }

                ui.add_space(MARGIN);
            });

            if new_amount != current_amount {
                tile_entity
                    .send_message(TileEntityMsg::SetData(
                        "amount".to_owned(),
                        Data::Amount(new_amount),
                    ))
                    .unwrap();
            }

            if new_script != current_script {
                if let Some(script) = new_script {
                    tile_entity
                        .send_message(TileEntityMsg::SetData(
                            "script".to_owned(),
                            Data::Id(script),
                        ))
                        .unwrap();
                    tile_entity
                        .send_message(TileEntityMsg::RemoveData("buffer"))
                        .unwrap();
                }
            }

            if new_storage != current_storage {
                if let Some(storage) = new_storage {
                    tile_entity
                        .send_message(TileEntityMsg::SetData(
                            "storage".to_owned(),
                            Data::Id(storage),
                        ))
                        .unwrap();
                    tile_entity
                        .send_message(TileEntityMsg::RemoveData("buffer"))
                        .unwrap();
                }
            }

            if new_target_coord != current_target_coord {
                if let Some(target_coord) = new_target_coord {
                    game.send_message(GameMsg::ForwardMsgToTile(
                        config_open,
                        TileEntityMsg::SetData("target".to_owned(), Data::Coord(target_coord)),
                    ))
                    .unwrap();
                    game.send_message(GameMsg::SignalTilesUpdated).unwrap();
                } else {
                    game.send_message(GameMsg::ForwardMsgToTile(
                        config_open,
                        TileEntityMsg::RemoveData("target"),
                    ))
                    .unwrap();
                    game.send_message(GameMsg::SignalTilesUpdated).unwrap();
                }
            }
        }
    }
}
pub fn line(a: DPoint2, b: DPoint2, color: Rgba) -> Vec<GameVertex> {
    let v = b - a;
    let l = a.distance(b) * 128.0;
    let w = cgmath::vec2(-v.y / l, v.x / l);

    let a0 = (a + w).cast::<Float>().unwrap();
    let a1 = (b + w).cast::<Float>().unwrap();
    let b0 = (b - w).cast::<Float>().unwrap();
    let b1 = (a - w).cast::<Float>().unwrap();

    let mut line = vec![];

    Quad::new(
        GameVertex {
            pos: [a0.x, a0.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [a1.x, a1.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [b0.x, b0.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [b1.x, b1.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
    )
    .emit_triangles(|v| line.append(&mut vec![v.x, v.y, v.z]));

    line
}
