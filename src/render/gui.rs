use std::f32::consts::FRAC_PI_4;
use std::fs;
use std::sync::Arc;

use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures::executor::block_on;
use genmesh::{EmitTriangles, Quad};
use ractor::ActorRef;
use tokio::runtime::Runtime;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::image::SampleCount::Sample4;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};

use automancy_defs::cg::{perspective, DPoint2, DPoint3, Double, Float, Matrix4, Vector3};
use automancy_defs::cgmath::{point2, point3, vec3, MetricSpace};
use automancy_defs::coord::{TileCoord, TileHex};
use automancy_defs::egui::epaint::Shadow;
use automancy_defs::egui::style::{WidgetVisuals, Widgets};
use automancy_defs::egui::FontFamily::{Monospace, Proportional};
use automancy_defs::egui::{
    vec2, Align, Align2, Color32, CursorIcon, DragValue, FontData, FontDefinitions, FontId, Frame,
    Margin, PaintCallback, Rgba, RichText, Rounding, ScrollArea, Sense, Stroke, Style, TextStyle,
    TopBottomPanel, Ui, Visuals, Window,
};
use automancy_defs::egui_winit_vulkano::{CallbackFn, Gui, GuiConfig};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::traits::HexDirection;
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameVertex, InstanceData, LightInfo};
use automancy_defs::winit::event_loop::{ControlFlow, EventLoop};
use automancy_defs::{cgmath, colors, log};
use automancy_resources::data::item::Item;
use automancy_resources::data::Data;
use automancy_resources::error::{error_to_key, error_to_string};
use automancy_resources::{format, unix_to_formatted_time, ResourceManager};

use crate::game::map::{Map, MapInfo};
use crate::game::run::setup::GameSetup;
use crate::game::state::GameMsg;
use crate::game::tile::entity::{TileEntityMsg, TileModifier};
use crate::render::camera::hex_to_normalized;
use crate::render::event::{shutdown_graceful, EventLoopStorage};
use crate::render::gpu::Gpu;
use crate::render::renderer::Renderer;
use crate::render::{gpu, gui};
use crate::{IOSEVKA_FONT, VERSION};

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum GuiState {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Clone)]
pub enum PopupState {
    None,
    MapCreate,
    MapDeleteConfirmation(MapInfo),
}

/// Initialize the font families.
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

/// Initialize the GUI style.
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

/// Initializes the GUI.
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

/// Creates a default frame.
pub fn default_frame() -> Frame {
    Frame::none()
        .fill(colors::WHITE.multiply(0.65).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: colors::DARK_GRAY.multiply(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
}

/// Draws the tile selection.
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
            setup
                .resource_man
                .registry
                .tile(*id)
                .unwrap()
                .models
                .get(*selected_tile_modifiers.get(id).unwrap_or(&0) as usize)
                .map(|model| (*id, *model))
        })
        .for_each(|(id, model)| {
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
            let resource_man = setup.resource_man.clone();

            let callback = PaintCallback {
                rect,
                callback: Arc::new(CallbackFn::new(move |_info, context| {
                    let instance = (
                        InstanceData::default().with_model_matrix(matrix).into(),
                        model,
                    );

                    let light_info = Buffer::from_data(
                        &context.resources.memory_allocator,
                        BufferCreateInfo {
                            usage: BufferUsage::VERTEX_BUFFER,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            usage: MemoryUsage::Upload,
                            ..Default::default()
                        },
                        LightInfo {
                            light_pos: [0.0, 0.0, 12.0],
                            light_color: [1.0; 4],
                        },
                    )
                    .unwrap();

                    if let Some((indirect_commands, instance_buffer)) = gpu::indirect_instance(
                        &context.resources.memory_allocator,
                        &resource_man,
                        &[instance],
                    ) {
                        context
                            .builder
                            .bind_pipeline_graphics(pipeline.clone())
                            .bind_vertex_buffers(
                                0,
                                (vertex_buffer.clone(), instance_buffer, light_info),
                            )
                            .bind_index_buffer(index_buffer.clone())
                            .draw_indexed_indirect(indirect_commands)
                            .unwrap();
                    }
                })),
            };

            ui.painter().add(callback);
        });
}

/// Creates the tile selection GUI.
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

/// Draws the tile info GUI.
pub fn tile_info(runtime: &Runtime, setup: &GameSetup, gui: &Gui) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_info]
            .to_string(),
    )
    .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.colored_label(colors::DARK_GRAY, setup.camera.pointing_at.to_string());

        let tile_entity = runtime
            .block_on(setup.game.call(
                |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
                None,
            ))
            .unwrap()
            .unwrap();

        let tile = runtime
            .block_on(setup.game.call(
                |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
                None,
            ))
            .unwrap()
            .unwrap();

        if let Some((tile_entity, (id, _))) = tile_entity.zip(tile) {
            ui.label(setup.resource_man.tile_name(&id));

            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            if let Some(inventory) = data
                .get(&setup.resource_man.registry.data_ids.buffer)
                .and_then(Data::as_inventory)
            {
                for (item, amount) in inventory.0.iter() {
                    ui.label(format!(
                        "{} - {}",
                        setup.resource_man.item_name(&item.id),
                        amount
                    ));
                }
            }
            //ui.label(format!("State: {}", ask(sys, &game, )))
        }
    });
}

/// Draws an error popup. Can only be called when there are errors in the queue!
pub fn error_popup(setup: &mut GameSetup, gui: &mut Gui) {
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
            });
        });
    });
}

/// Draws the debug menu (F3).
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
    let api_version = renderer
        .gpu
        .surface
        .instance()
        .max_api_version()
        .to_string();

    let fps = 1.0 / loop_store.elapsed.as_secs_f64();

    let reg_tiles = resource_man.registry.tiles.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    //let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.meshes.len();

    let map = runtime
        .block_on(game.call(GameMsg::GetMapInfo, Some(loop_store.elapsed)))
        .unwrap()
        .unwrap();

    let map_name = map.map_name;
    let data_size = map.data;
    let tile_count = map.tiles;

    Window::new(
        setup.resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].to_string(),
    )
    .resizable(false)
    .default_width(600.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.label(format!("FPS: {fps:.1}"));
        ui.label(format!("Device: {device_name} API {api_version}"));
        ui.label(format!(
            "ResourceMan: {reg_tiles}T {reg_items}I {tags}Ta {scripts}S {audio}A {meshes}M"
        ));
        ui.label(format!(
            "Map \"{map_name}\" ({map_name}.bin): {data_size}D {tile_count}T"
        ))
    });
}

/// Draws the main menu.
pub fn main_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    control_flow: &mut ControlFlow,
    loop_store: &mut EventLoopStorage,
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
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_play]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        setup.refresh_maps();
                        loop_store.gui_state = GuiState::MapLoad
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_options]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.gui_state = GuiState::Options
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_fedi]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        webbrowser::open("https://gamedev.lgbt/@automancy")
                            .expect("Failed to open web browser");
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_source]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        webbrowser::open("https://github.com/sorcerers-class/automancy")
                            .expect("Failed to open web browser");
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_exit]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        shutdown_graceful(setup, control_flow)
                            .expect("Failed to shutdown gracefully!");
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the pause menu.
pub fn pause_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new("Game Paused".to_string())
        .resizable(false)
        .default_width(175.0)
        .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
        .frame(default_frame().inner_margin(10.0))
        .movable(false)
        .show(&gui.context(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_unpause]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.gui_state = GuiState::Ingame
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_options]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.gui_state = GuiState::Options
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_exit]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        block_on(setup.game.call(
                            |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
                            None,
                        ))
                        .unwrap();
                        setup
                            .game
                            .send_message(GameMsg::LoadMap(
                                setup.resource_man.clone(),
                                ".mainmenu".to_string(),
                            ))
                            .unwrap();
                        renderer.reset_last_tiles_update();
                        loop_store.gui_state = GuiState::MainMenu
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the map loading menu.
pub fn map_load_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.load_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ScrollArea::vertical().max_height(225.0).show(ui, |ui| {
            let dirty = false;
            setup.maps.iter().for_each(|map| {
                let resource_man = setup.resource_man.clone();
                let time = unix_to_formatted_time(
                    map.save_time,
                    resource_man.translates.gui[&resource_man.registry.gui_ids.time_fmt].as_str(),
                );
                ui.group(|ui| {
                    ui.label(RichText::new(&map.map_name).heading());
                    ui.horizontal(|ui| {
                        ui.label(time);
                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_load]
                                    .to_string(),
                            )
                            .clicked()
                        {
                            setup
                                .game
                                .send_message(GameMsg::LoadMap(resource_man, map.map_name.clone()))
                                .unwrap();
                            renderer.reset_last_tiles_update();
                            loop_store.gui_state = GuiState::Ingame;
                        }
                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_delete]
                                    .to_string(),
                            )
                            .clicked()
                        {
                            loop_store.popup_state = PopupState::MapDeleteConfirmation(map.clone());
                        }
                    });
                });
            });
            if dirty {
                setup.refresh_maps();
            }
        });
        ui.label(format(
            setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.lbl_maps_loaded]
                .as_str(),
            &[setup.maps.len().to_string().as_str()],
        ));
        ui.horizontal(|ui| {
            if ui
                .button(
                    RichText::new(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_new_map]
                            .to_string(),
                    )
                    .heading(),
                )
                .clicked()
            {
                loop_store.popup_state = PopupState::MapCreate
            }
            if ui
                .button(
                    RichText::new(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_cancel]
                            .to_string(),
                    )
                    .heading(),
                )
                .clicked()
            {
                loop_store.gui_state = GuiState::MainMenu
            }
        });
    });
}

pub fn map_delete_confirmation(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    map: MapInfo,
) {
    let mut dirty = false;
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.delete_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.lbl_delete_map_confirm]
                .to_string(),
        );
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .to_string(),
            )
            .clicked()
        {
            fs::remove_file(format!("map/{}.bin", map.map_name)).unwrap();
            dirty = true;
            loop_store.popup_state = PopupState::None;
            log::info!("Deleted map {}!", map.map_name);
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .to_string(),
            )
            .clicked()
        {
            loop_store.popup_state = PopupState::None
        }
    });
    if dirty {
        setup.refresh_maps();
    }
}

/// Draws the map creation popup.
pub fn map_create_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.create_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut loop_store.filter);
        });
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .to_string(),
            )
            .clicked()
        {
            let name = Map::sanitize_name(loop_store.filter.clone());
            setup
                .game
                .send_message(GameMsg::LoadMap(setup.resource_man.clone(), name))
                .unwrap();
            renderer.reset_last_tiles_update();
            loop_store.filter.clear();
            loop_store.popup_state = PopupState::None;
            loop_store.gui_state = GuiState::Ingame
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .to_string(),
            )
            .clicked()
        {
            loop_store.popup_state = PopupState::None
        }
    });
}

/// Draws the options menu. TODO
pub fn options_menu(setup: &mut GameSetup, gui: &mut Gui, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.options].to_string(),
    )
    .resizable(false)
    .default_width(175.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.label("Not yet implemented");
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .to_string(),
            )
            .clicked()
        {
            loop_store.gui_state = GuiState::MainMenu
        }
    });
}

/// Draws the direction selector.
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

/// Draws a search bar.
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

/// Draws the targets UI.
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

pub fn draw_item(
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    renderer: &Renderer,
    item: Item,
    size: Float,
) {
    let model = if resource_man.meshes.contains_key(&item.model) {
        item.model
    } else {
        resource_man.registry.model_ids.items_missing
    };

    let (_, rect) = ui.allocate_space(vec2(size, size));

    let pipeline = renderer.gpu.gui_pipeline.clone();
    let vertex_buffer = renderer.gpu.alloc.vertex_buffer.clone();
    let index_buffer = renderer.gpu.alloc.index_buffer.clone();

    let callback = PaintCallback {
        rect,
        callback: Arc::new(CallbackFn::new(move |_info, context| {
            let instance = (InstanceData::default().into(), model);

            let light_info = Buffer::from_data(
                &context.resources.memory_allocator,
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                LightInfo {
                    light_pos: [0.0, 0.0, 2.0],
                    light_color: [1.0; 4],
                },
            )
            .unwrap();

            if let Some((indirect_commands, instance_buffer)) = gpu::indirect_instance(
                &context.resources.memory_allocator,
                &resource_man,
                &[instance],
            ) {
                context
                    .builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer, light_info))
                    .bind_index_buffer(index_buffer.clone())
                    .draw_indexed_indirect(indirect_commands)
                    .unwrap();
            }
        })),
    };

    ui.painter().add(callback);
}

/// Draws the tile configuration menu.
pub fn tile_config(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &Renderer,
    gui: &Gui,
    extra_vertices: &mut Vec<GameVertex>,
) {
    let window_size = setup.window.inner_size();

    if let Some(config_open) = loop_store.config_open {
        let tile = runtime
            .block_on(
                setup
                    .game
                    .call(|reply| GameMsg::GetTile(config_open, reply), None),
            )
            .unwrap()
            .unwrap();

        let tile_entity = runtime
            .block_on(
                setup
                    .game
                    .call(|reply| GameMsg::GetTileEntity(config_open, reply), None),
            )
            .unwrap()
            .unwrap();

        if let Some(((id, _), tile_entity)) = tile.zip(tile_entity) {
            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            let current_amount = data
                .get(&setup.resource_man.registry.data_ids.amount)
                .and_then(Data::as_amount)
                .cloned()
                .unwrap_or(0);
            let mut new_amount = current_amount;

            let current_script = data
                .get(&setup.resource_man.registry.data_ids.script)
                .and_then(Data::as_id)
                .cloned();
            let mut new_script = current_script;

            let current_storage = data
                .get(&setup.resource_man.registry.data_ids.storage)
                .and_then(Data::as_id)
                .cloned();
            let mut new_storage = current_storage;

            let current_target_coord = data
                .get(&setup.resource_man.registry.data_ids.target)
                .and_then(Data::as_coord)
                .cloned();
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

                let tile_info = setup.resource_man.registry.tile(id).unwrap();

                if let Some(scripts) = tile_info
                    .data
                    .get(&setup.resource_man.registry.data_ids.scripts)
                    .and_then(Data::as_vec_id)
                {
                    ui.add_space(MARGIN);

                    ui.label(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.tile_config_script]
                            .as_str(),
                    );

                    ui.vertical(|ui| {
                        const SIZE: Float = 32.0;

                        if let Some(script) =
                            new_script.and_then(|id| setup.resource_man.registry.script(id))
                        {
                            if let Some(inputs) = &script.instructions.inputs {
                                inputs.iter().for_each(|item_stack| {
                                    ui.horizontal(|ui| {
                                        ui.set_height(SIZE);

                                        ui.label(" + ");
                                        draw_item(
                                            ui,
                                            setup.resource_man.clone(),
                                            renderer,
                                            item_stack.item,
                                            SIZE,
                                        );
                                        ui.label(format!(
                                            "{} ({})",
                                            setup.resource_man.item_name(&item_stack.item.id),
                                            item_stack.amount
                                        ));
                                    });
                                })
                            }

                            ui.horizontal(|ui| {
                                ui.set_height(SIZE);

                                ui.label("=> ");
                                draw_item(
                                    ui,
                                    setup.resource_man.clone(),
                                    renderer,
                                    script.instructions.output.item,
                                    SIZE,
                                );
                                ui.label(format!(
                                    "{} ({})",
                                    setup
                                        .resource_man
                                        .item_name(&script.instructions.output.item.id),
                                    script.instructions.output.amount
                                ));
                            });
                        }
                    });

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

                if let Some(Data::Id(storage_type)) = tile_info
                    .data
                    .get(&setup.resource_man.registry.data_ids.storage_type)
                {
                    let storage_text = if let Some(item) = new_storage
                        .as_ref()
                        .and_then(|id| setup.resource_man.registry.item(*id))
                    {
                        setup.resource_man.item_name(&item.id).to_string()
                    } else {
                        setup.resource_man.translates.none.to_string()
                    };

                    let items = setup
                        .resource_man
                        .get_items(*storage_type, &mut loop_store.tag_cache)
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

                if id == setup.resource_man.registry.tile_ids.master_node {
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

                if id == setup.resource_man.registry.tile_ids.node {
                    if let Some(tile_entity) = runtime
                        .block_on(
                            setup
                                .game
                                .call(|reply| GameMsg::GetTileEntity(config_open, reply), None),
                        )
                        .unwrap()
                        .unwrap()
                    {
                        let result = runtime
                            .block_on(tile_entity.call(
                                |reply| {
                                    TileEntityMsg::GetDataValue(
                                        setup.resource_man.registry.data_ids.link,
                                        reply,
                                    )
                                },
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

                if setup.resource_man.registry.tile(id).unwrap().targeted {
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
                        setup.resource_man.registry.data_ids.amount,
                        Data::Amount(new_amount),
                    ))
                    .unwrap();
            }

            if new_script != current_script {
                if let Some(script) = new_script {
                    tile_entity
                        .send_message(TileEntityMsg::SetData(
                            setup.resource_man.registry.data_ids.script,
                            Data::Id(script),
                        ))
                        .unwrap();
                    tile_entity
                        .send_message(TileEntityMsg::RemoveData(
                            setup.resource_man.registry.data_ids.buffer,
                        ))
                        .unwrap();
                }
            }

            if new_storage != current_storage {
                if let Some(storage) = new_storage {
                    tile_entity
                        .send_message(TileEntityMsg::SetData(
                            setup.resource_man.registry.data_ids.storage,
                            Data::Id(storage),
                        ))
                        .unwrap();
                    tile_entity
                        .send_message(TileEntityMsg::RemoveData(
                            setup.resource_man.registry.data_ids.buffer,
                        ))
                        .unwrap();
                }
            }

            if new_target_coord != current_target_coord {
                if let Some(target_coord) = new_target_coord {
                    setup
                        .game
                        .send_message(GameMsg::ForwardMsgToTile(
                            config_open,
                            TileEntityMsg::SetData(
                                setup.resource_man.registry.data_ids.target,
                                Data::Coord(target_coord),
                            ),
                        ))
                        .unwrap();
                    setup
                        .game
                        .send_message(GameMsg::SignalTilesUpdated)
                        .unwrap();
                } else {
                    setup
                        .game
                        .send_message(GameMsg::ForwardMsgToTile(
                            config_open,
                            TileEntityMsg::RemoveData(setup.resource_man.registry.data_ids.target),
                        ))
                        .unwrap();
                    setup
                        .game
                        .send_message(GameMsg::SignalTilesUpdated)
                        .unwrap();
                }
            }
        }
    }
}

/// Draws a line overlay.
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
