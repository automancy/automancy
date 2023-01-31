use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use cgmath::{point3, vec3};
use egui::{CursorIcon, PaintCallback, Sense, Ui, vec2};
use egui_winit_vulkano::CallbackFn;
use futures::channel::mpsc;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::traits::HexDirection;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::DrawIndexedIndirectCommand;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};

use crate::data::id::Id;
use crate::game::tile::TileUnit;
use crate::render::data::UniformBufferObject;
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::util::cg::{Matrix4, perspective, Vector3};
use crate::util::init::InitData;
use crate::util::resource::ResourceType;

fn tile_ui(size: f32, id: Id, faces_index: usize, mut channel: mpsc::Sender<Id>, ui: &mut Ui, init_data: Arc<InitData>, gpu: Arc<Gpu>, gui_pipeline: Arc<GraphicsPipeline>) -> PaintCallback {
    let (rect, response) = ui.allocate_exact_size(
        vec2(size, size),
        Sense::click(),
    );

    response.clone().on_hover_text(init_data.resource_man.tile_name(&id));
    response.clone().on_hover_cursor(CursorIcon::Grab);

    let hover = if response.clone().hovered() {
        ui.ctx().animate_value_with_time(ui.next_auto_id(), 1.0, 0.3)
    } else {
        ui.ctx().animate_value_with_time(ui.next_auto_id(), 0.0, 0.3)
    };

    if response.clicked() {
        channel.try_send(id).unwrap();
    }

    let pos = point3(0.0, 0.0, 1.0 - (0.5 * hover));
    let eye = point3(pos.x, pos.y, pos.z - 0.3);
    let matrix = perspective(FRAC_PI_4,1.0, 0.01, 10.0)
        * Matrix4::from_translation(vec3(0.0, 0.0, 2.0))
        * Matrix4::look_to_rh(eye, vec3(0.0, 1.0 - pos.z, 1.0), Vector3::unit_y());

    PaintCallback {
        rect,
        callback: Arc::new(CallbackFn::new(move |_info, context| {
            let faces = &init_data.resource_man.all_faces[faces_index];

            let uniform_buffer = gpu::uniform_buffer(&context.resources.memory_allocator);

            let ubo_layout = gui_pipeline.layout().set_layouts()[0].clone();

            let ubo = UniformBufferObject {
                matrix: matrix.into(),
            };

            *uniform_buffer.write().unwrap() = ubo;

            let ubo_set = PersistentDescriptorSet::new(
                context.resources.descriptor_set_allocator,
                ubo_layout,
                [WriteDescriptorSet::buffer(
                    0,
                    uniform_buffer.clone(),
                )],
            ).unwrap();

            context.builder
                .bind_pipeline_graphics(gui_pipeline.clone())
                .bind_vertex_buffers(0, gpu.vertex_buffer.clone())
                .bind_index_buffer(gpu.index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    gui_pipeline.layout().clone(),
                    0,
                    ubo_set,
                );

            let commands = faces
                .iter()
                .map(|face| {
                    DrawIndexedIndirectCommand {
                        index_count: face.size,
                        instance_count: 1,
                        first_index: face.offset,
                        vertex_offset: 0,
                        first_instance: 0
                    }
                });

            let indirect_buffer = gpu::cpu_accessible_buffer(
                &context.resources.memory_allocator,
                commands,
                BufferUsage {
                    indirect_buffer: true,
                    ..Default::default()
                }
            );

            context.builder.draw_indexed_indirect(indirect_buffer).unwrap();
        })),
    }
}

pub fn render_tile_selection(ui: &mut Ui, init_data: Arc<InitData>, channel: mpsc::Sender<Id>, gpu: Arc<Gpu>, gui_pipeline: Arc<GraphicsPipeline>) {
    let size = ui.available_height();
    let resource_man = &init_data.clone().resource_man;

    resource_man.ordered_ids
        .iter()
        .flat_map(|id| {
            let resource = &resource_man.resources[id];

            if resource.resource_type == ResourceType::Model {
                return None;
            }

            resource.faces_index.map(|v| (id.clone(), v))
        })
        .for_each(|(id, faces_index)| {
            let callback = tile_ui(size, id, faces_index, channel.clone(), ui, init_data.clone(), gpu.clone(), gui_pipeline.clone());

            ui.painter().add(callback.clone());
        });
}

pub fn add_direction(ui: &mut Ui, target_coord: &mut Option<Hex<TileUnit>>, n: usize) {
    let coord = Hex::<TileUnit>::NEIGHBORS[(n + 2) % 6];
    let coord = Some(coord);

    ui.selectable_value(target_coord, coord, match n {
        0 => "↗",
        1 => "➡",
        2 => "↘",
        3 => "↙",
        4 => "⬅",
        5 => "↖",
        _ => "",
    });
}