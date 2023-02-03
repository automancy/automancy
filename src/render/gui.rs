use std::collections::HashMap;
use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use cgmath::{point3, vec3};
use egui::epaint::Shadow;
use egui::style::{Margin, WidgetVisuals, Widgets};
use egui::FontFamily::{Monospace, Proportional};
use egui::{
    vec2, Align, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontId, Frame,
    PaintCallback, Rounding, ScrollArea, Sense, Stroke, Style, TextStyle, TopBottomPanel, Ui,
    Visuals, Window,
};
use egui_winit_vulkano::{CallbackFn, Gui};
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures_executor::block_on;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::traits::HexDirection;
use riker::actors::{ActorRef, ActorSystem};
use riker_patterns::ask::ask;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use winit::event_loop::EventLoop;

use crate::game::data::Data;
use crate::game::game::GameMsg;
use crate::game::tile::{TileCoord, TileEntityMsg, TileUnit};
use crate::render::data::{GuiUBO, InstanceData};
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::render::renderer::Renderer;
use crate::util::cg::{perspective, Matrix4, Vector3};
use crate::util::colors::Color;
use crate::util::id::Id;
use crate::util::resource::ResourceManager;
use crate::util::resource::TileType;
use crate::IOSEVKA_FONT;

fn init_fonts(gui: &Gui) {
    let mut fonts = FontDefinitions::default();
    let iosevka = "iosevka".to_owned();

    fonts
        .font_data
        .insert(iosevka.clone(), FontData::from_static(IOSEVKA_FONT));

    fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, iosevka.clone());
    fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, iosevka);

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
                    bg_fill: Color32::from_gray(190),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    bg_fill: Color32::from_gray(180), // button background
                    bg_stroke: Default::default(),
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    bg_fill: Color32::from_gray(170),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 1.0,
                },
                active: WidgetVisuals {
                    bg_fill: Color32::from_gray(160),
                    bg_stroke: Stroke::new(1.0, Color32::BLACK),
                    fg_stroke: Stroke::new(2.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 1.0,
                },
                open: WidgetVisuals {
                    bg_fill: Color32::from_gray(170),
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
        Some(gpu.alloc.swapchain.image_format()),
        gpu.queue.clone(),
        gpu.gui_subpass.clone(),
    );

    init_fonts(&gui);
    init_styles(&gui);

    gui
}

pub fn default_frame() -> Frame {
    Frame::none()
        .fill(Color::WHITE.with_alpha(0.7).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: Color::DARK_GRAY.with_alpha(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
}

fn tile_paint(
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    gpu: &Gpu,
    size: f32,
    id: Id,
    faces_index: usize,
    selection_send: &mut mpsc::Sender<Id>,
) -> PaintCallback {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());

    response.clone().on_hover_text(resource_man.tile_name(&id));
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

    let pos = point3(0.0, 0.0, 1.0 - (0.5 * hover));
    let eye = point3(pos.x, pos.y, pos.z - 0.3);
    let matrix = perspective(FRAC_PI_4, 1.0, 0.01, 10.0)
        * Matrix4::from_translation(vec3(0.0, 0.0, 2.0))
        * Matrix4::look_to_rh(eye, vec3(0.0, 1.0 - pos.z, 1.0), Vector3::unit_y());

    let pipeline = gpu.gui_pipeline.clone();
    let vertex_buffer = gpu.alloc.vertex_buffer.clone();
    let index_buffer = gpu.alloc.index_buffer.clone();
    let ubo_layout = pipeline.layout().set_layouts()[0].clone();

    PaintCallback {
        rect,
        callback: Arc::new(CallbackFn::new(move |_info, context| {
            let uniform_buffer = gpu::uniform_buffer(&context.resources.memory_allocator);

            let ubo = GuiUBO {
                matrix: matrix.into(),
            };

            *uniform_buffer.write().unwrap() = ubo;

            let ubo_set = PersistentDescriptorSet::new(
                context.resources.descriptor_set_allocator,
                ubo_layout.clone(),
                [WriteDescriptorSet::buffer(0, uniform_buffer)],
            )
            .unwrap();

            let instance = InstanceData::new().faces_index(faces_index);

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
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    selected_tile_states: &HashMap<Id, usize>,
    gpu: &Gpu,
    mut selection_send: mpsc::Sender<Id>,
) {
    let size = ui.available_height();

    resource_man
        .ordered_ids
        .iter()
        .flat_map(|id| {
            let resource = &resource_man.tiles[id];

            if resource.tile_type == TileType::Model {
                return None;
            }

            resource
                .faces_indices
                .get(*selected_tile_states.get(id).unwrap_or(&0))
                .map(|v| (*id, *v))
        })
        .for_each(|(id, faces_index)| {
            let callback = tile_paint(
                ui,
                resource_man.clone(),
                gpu,
                size,
                id,
                faces_index,
                &mut selection_send,
            );

            ui.painter().add(callback);
        });
}

pub fn tile_selections(
    gui: &mut Gui,
    resource_man: Arc<ResourceManager>,
    selected_tile_states: &HashMap<Id, usize>,
    renderer: &Renderer,
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
                            ui,
                            resource_man.clone(),
                            selected_tile_states,
                            &renderer.gpu,
                            selection_send,
                        );
                    });
                });
        });
}

pub fn tile_info(
    gui: &mut Gui,
    resource_man: Arc<ResourceManager>,
    sys: &ActorSystem,
    game: ActorRef<GameMsg>,
    pointing_at: TileCoord,
) {
    Window::new("Tile Info")
        .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
        .resizable(false)
        .default_width(300.0)
        .frame(default_frame().inner_margin(Margin::same(10.0)))
        .show(&gui.context(), |ui| {
            ui.colored_label(Color::DARK_GRAY, pointing_at.to_string());

            let result: Option<(Id, ActorRef<TileEntityMsg>, usize)> =
                block_on(ask(sys, &game, GameMsg::GetTile(pointing_at)));

            if let Some((id, tile, _)) = result {
                ui.label(resource_man.tile_name(&id));
                let data: Data = block_on(ask(sys, &tile, TileEntityMsg::GetData));

                for (id, amount) in data.0.iter() {
                    ui.label(format!("{} - {}", resource_man.item_name(id), amount));
                }
                //ui.label(format!("State: {}", ask(sys, &game, )))
            }
        });
}

pub fn add_direction(ui: &mut Ui, target_coord: &mut Option<Hex<TileUnit>>, n: usize) {
    let coord = Hex::<TileUnit>::NEIGHBORS[(n + 2) % 6];
    let coord = Some(coord);

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

pub fn scripts(
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    fuse: &Fuse,
    scripts: Vec<Id>,
    new_script: &mut Option<Id>,
    script_filter: &mut String,
) {
    ui.text_edit_singleline(script_filter);

    ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
        ui.set_width(ui.available_width());

        let scripts = if !script_filter.is_empty() {
            let mut filtered = scripts
                .into_iter()
                .flat_map(|id| {
                    let result = fuse
                        .search_text_in_string(script_filter, resource_man.item_name(&id).as_str());

                    Some(id).zip(result.map(|v| v.score))
                })
                .collect::<Vec<_>>();

            filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
        } else {
            scripts
        };

        scripts.iter().for_each(|script| {
            ui.radio_value(new_script, Some(*script), resource_man.item_name(script));
        })
    });
}

pub fn targets(ui: &mut Ui, new_target_coord: &mut Option<Hex<TileUnit>>) {
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
