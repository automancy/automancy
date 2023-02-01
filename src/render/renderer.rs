use std::f32::consts::PI;
use std::sync::Arc;

use cgmath::SquareMatrix;
use egui_winit_vulkano::Gui;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};
use hexagon_tiles::point::point;
use hexagon_tiles::traits::HexRound;
use vulkano::buffer::BufferUsage;

use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage,
    RenderPassBeginInfo, SubpassContents,
};

use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::format::ClearValue;

use vulkano::memory::allocator::FastMemoryAllocator;
use vulkano::pipeline::graphics::viewport::Scissor;
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::swapchain::{acquire_next_image, AcquireError};
use vulkano::sync;
use vulkano::sync::GpuFuture;

use crate::game::map::MapRenderInfo;
use crate::game::tile::{TileCoord, TileUnit};
use crate::render::camera::{CameraState, FAR};
use crate::render::data::{InstanceData, UniformBufferObject, Vertex, RENDER_LAYOUT};
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::util::cg::{matrix, Matrix4, Num};
use crate::util::colors::Color;
use crate::util::id::Id;
use crate::util::init::InitData;

pub struct Renderer {
    init_data: Arc<InitData>,

    pub recreate_swapchain: bool,

    pub gpu: Gpu,

    previous_frame_end: Option<Box<dyn GpuFuture + Send + Sync>>,
}

impl Renderer {
    pub fn new(init_data: Arc<InitData>, gpu: Gpu) -> Self {
        let device = gpu.device.clone();

        Self {
            init_data,

            recreate_swapchain: false,

            gpu,

            previous_frame_end: Some(sync::now(device).boxed_send_sync()),
        }
    }
}

impl Renderer {
    pub fn render(
        &mut self,
        map_render_info: MapRenderInfo,
        camera_state: CameraState,
        none: Id,
        mut extra_instances: Vec<InstanceData>,
        extra_vertices: Vec<Vertex>,
        gui: &mut Gui,
    ) {
        let (width, height) = gpu::window_size(&self.gpu.window);
        let aspect = width / height;

        let camera_pos = camera_state.pos;

        let instances = {
            let pos = point(camera_pos.x, camera_pos.y);
            let pos = pixel_to_hex(RENDER_LAYOUT, pos).round();

            // TODO move this constant
            const RANGE: TileUnit = 32;

            let o = Hex::new(RANGE, RANGE);

            let min = pos - o;
            let max = pos + o;

            let none = InstanceData::new().faces_index(
                self.init_data.resource_man.resources[&none]
                    .faces_index
                    .unwrap(),
            );

            let mut instances = map_render_info.instances;

            for q in min.q()..max.q() {
                for r in min.r()..max.r() {
                    let pos = Hex::new(q, r);
                    let p = hex_to_pixel(RENDER_LAYOUT, pos);

                    instances.entry(TileCoord(pos)).or_insert_with(|| {
                        none.position_offset([p.x as Num, p.y as Num, FAR as Num])
                    });
                }
            }

            if camera_state.is_at_max_height() {
                instances
                    .get_mut(&camera_state.pointing_at)
                    .map(|instance| {
                        *instance = instance
                            .add_position_offset([0.0, 0.0, 0.0001])
                            .color_offset(Color::ORANGE.with_alpha(0.5).into())
                    });
            }

            let mut instances = instances.into_values().collect::<Vec<_>>();

            instances.sort_by_key(|v| v.faces_index);

            instances.append(&mut extra_instances);

            instances
        };

        self.inner_render(
            matrix(camera_pos.cast::<Num>().unwrap(), aspect as Num, PI),
            &instances,
            extra_vertices,
            gui,
        );
    }

    fn inner_render(
        &mut self,
        matrix: Matrix4,
        instances: &[InstanceData],
        extra_vertices: Vec<Vertex>,
        gui: &mut Gui,
    ) {
        let dimensions = gpu::window_size_u32(&self.gpu.window);

        if dimensions[0] == 0 || dimensions[1] == 0 {
            return;
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        let allocator = FastMemoryAllocator::new_default(self.gpu.device.clone());

        self.gpu
            .resize_images(&allocator, dimensions, &mut self.recreate_swapchain);

        if self.recreate_swapchain {
            self.gpu
                .recreate_swapchain(dimensions, &mut self.recreate_swapchain);
        }

        let (image_num, suboptimal, acquire_future) = {
            match acquire_next_image(self.gpu.alloc.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {:?}", e),
            }
        };
        if suboptimal {
            self.recreate_swapchain = true;
        }
        let image_num = image_num as usize;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.gpu.alloc.command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        None,
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Depth(1.0)),
                        Some(ClearValue::Depth(1.0)),
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.gpu.framebuffers[image_num].clone())
                },
                SubpassContents::SecondaryCommandBuffers,
            )
            .unwrap();

        let mut game_builder = AutoCommandBufferBuilder::secondary(
            &self.gpu.alloc.command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.gpu.game_subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();

        game_builder.set_viewport(0, [gpu::viewport(&self.gpu.window)]);

        let indirect_instance = gpu::indirect_instance(&allocator, &self.init_data, instances);

        let ubo = UniformBufferObject {
            matrix: matrix.into(),
        };

        *self.gpu.alloc.game_uniform_buffer.write().unwrap() = ubo;

        let ubo_layout = self.gpu.game_pipeline.layout().set_layouts()[0].clone();

        let ubo_set = PersistentDescriptorSet::new(
            &self.gpu.alloc.descriptor_allocator,
            ubo_layout.clone(),
            [WriteDescriptorSet::buffer(
                0,
                self.gpu.alloc.game_uniform_buffer.clone(),
            )],
        )
        .unwrap();

        if let Some((indirect_buffer, instance_buffer)) = indirect_instance {
            game_builder
                .bind_pipeline_graphics(self.gpu.game_pipeline.clone())
                .bind_vertex_buffers(0, (self.gpu.alloc.vertex_buffer.clone(), instance_buffer))
                .bind_index_buffer(self.gpu.alloc.index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.gpu.game_pipeline.layout().clone(),
                    0,
                    ubo_set,
                )
                .draw_indexed_indirect(indirect_buffer)
                .unwrap();
        }

        // game
        builder
            .execute_commands(game_builder.build().unwrap())
            .unwrap();

        builder
            .next_subpass(SubpassContents::SecondaryCommandBuffers)
            .unwrap();

        // extra gui
        if !extra_vertices.is_empty() {
            let ubo = UniformBufferObject {
                matrix: Matrix4::identity().into(),
            };

            *self.gpu.alloc.gui_uniform_buffer.write().unwrap() = ubo;

            let gui_ubo_set = PersistentDescriptorSet::new(
                &self.gpu.alloc.descriptor_allocator,
                ubo_layout.clone(),
                [WriteDescriptorSet::buffer(
                    0,
                    self.gpu.alloc.gui_uniform_buffer.clone(),
                )],
            )
            .unwrap();

            let mut gui_builder = AutoCommandBufferBuilder::secondary(
                &self.gpu.alloc.command_allocator,
                self.gpu.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
                CommandBufferInheritanceInfo {
                    render_pass: Some(self.gpu.gui_subpass.clone().into()),
                    ..Default::default()
                },
            )
            .unwrap();

            let vertex_count = extra_vertices.len();

            let extra_vertex_buffer = gpu::cpu_accessible_buffer(
                &allocator,
                extra_vertices.into_iter(),
                BufferUsage {
                    vertex_buffer: true,
                    ..Default::default()
                },
            );

            gui_builder
                .set_viewport(0, [gpu::viewport(&self.gpu.window)])
                .set_scissor(0, [Scissor::irrelevant()])
                .bind_pipeline_graphics(self.gpu.gui_pipeline.clone())
                .bind_vertex_buffers(0, extra_vertex_buffer)
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.gpu.gui_pipeline.layout().clone(),
                    0,
                    gui_ubo_set,
                );
            gui_builder.draw(vertex_count as u32, 1, 0, 0).unwrap();
            builder
                .execute_commands(gui_builder.build().unwrap())
                .unwrap();
        }

        // egui
        let egui_command_buffer = gui.draw_on_subpass_image(dimensions);
        builder.execute_commands(egui_command_buffer).unwrap();

        // end
        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();
        self.gpu.commit_commands(
            image_num,
            acquire_future,
            command_buffer,
            &mut self.previous_frame_end,
            &mut self.recreate_swapchain,
        );
    }
}
