use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use cgmath::{point3, vec3};
use egui::epaint::Shadow;
use egui::style::{WidgetVisuals, Widgets};
use egui::FontFamily::{Monospace, Proportional};
use egui::{
    vec2, Color32, CursorIcon, FontData, FontDefinitions, FontId, Frame, PaintCallback, Rounding,
    Sense, Stroke, Style, TextStyle, Ui, Visuals,
};
use egui_winit_vulkano::{CallbackFn, Gui};
use futures::channel::mpsc;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::traits::HexDirection;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::DrawIndexedIndirectCommand;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};

use winit::event_loop::EventLoop;

use crate::game::tile::TileUnit;
use crate::render::data::UniformBufferObject;
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::util::cg::{perspective, Matrix4, Vector3};
use crate::util::colors::Color;
use crate::util::id::Id;
use crate::util::init::InitData;
use crate::util::resource::ResourceType;
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
        .insert(0, iosevka.clone());

    gui.context().set_fonts(fonts);
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

pub fn init_styles(gui: &Gui) {
    gui.clone().context().set_style(Style {
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

fn tile_ui(
    size: f32,
    id: Id,
    faces_index: usize,
    init_data: Arc<InitData>,
    gpu: &Gpu,
    channel: &mut mpsc::Sender<Id>,
    ui: &mut Ui,
) -> PaintCallback {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());

    response
        .clone()
        .on_hover_text(init_data.resource_man.tile_name(&id));
    response.clone().on_hover_cursor(CursorIcon::Grab);

    let hover = if response.clone().hovered() {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 1.0, 0.3)
    } else {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 0.0, 0.3)
    };

    if response.clicked() {
        channel.try_send(id).unwrap();
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
            let faces = &init_data.resource_man.all_faces[faces_index];

            let uniform_buffer = gpu::uniform_buffer(&context.resources.memory_allocator);

            let ubo = UniformBufferObject {
                matrix: matrix.into(),
            };

            *uniform_buffer.write().unwrap() = ubo;

            let ubo_set = PersistentDescriptorSet::new(
                context.resources.descriptor_set_allocator,
                ubo_layout.clone(),
                [WriteDescriptorSet::buffer(0, uniform_buffer.clone())],
            )
            .unwrap();

            context
                .builder
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .bind_index_buffer(index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    ubo_set.clone(),
                );

            let commands = faces.iter().map(|face| DrawIndexedIndirectCommand {
                index_count: face.size,
                instance_count: 1,
                first_index: face.offset,
                vertex_offset: 0,
                first_instance: 0,
            });

            let indirect_buffer = gpu::cpu_accessible_buffer(
                &context.resources.memory_allocator,
                commands,
                BufferUsage {
                    indirect_buffer: true,
                    ..Default::default()
                },
            );

            context
                .builder
                .draw_indexed_indirect(indirect_buffer)
                .unwrap();
        })),
    }
}

pub fn render_tile_selection(
    ui: &mut Ui,
    init_data: Arc<InitData>,
    gpu: &Gpu,
    mut channel: mpsc::Sender<Id>,
) {
    let size = ui.available_height();
    let resource_man = &init_data.clone().resource_man;

    resource_man
        .ordered_ids
        .iter()
        .flat_map(|id| {
            let resource = &resource_man.resources[id];

            if resource.resource_type == ResourceType::Model {
                return None;
            }

            resource.faces_index.map(|v| (id.clone(), v))
        })
        .for_each(|(id, faces_index)| {
            let callback = tile_ui(
                size,
                id,
                faces_index,
                init_data.clone(),
                &gpu,
                &mut channel,
                ui,
            );

            ui.painter().add(callback.clone());
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
