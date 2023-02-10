use std::f32::consts::PI;
use std::sync::Arc;

use egui_winit_vulkano::Gui;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};
use hexagon_tiles::point::point;
use hexagon_tiles::traits::HexRound;

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
use crate::game::tile::coord::TileUnit;
use crate::render::camera::{CameraState, FAR};
use crate::render::data::{GameUBO, GuiUBO, InstanceData, Vertex, RENDER_LAYOUT};
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::resource::ResourceManager;
use crate::util::cg::{actual_pos, eye, matrix, Matrix4, Num, Point3};
use crate::util::colors;
use crate::util::colors::WithAlpha;

pub struct Renderer {
    resource_man: Arc<ResourceManager>,

    pub recreate_swapchain: bool,

    pub gpu: Gpu,

    previous_frame_end: Option<Box<dyn GpuFuture + Send + Sync>>,
}

impl Renderer {
    pub fn new(resource_man: Arc<ResourceManager>, gpu: Gpu) -> Self {
        let device = gpu.device.clone();

        Self {
            resource_man,

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
        gui_instances: Vec<InstanceData>,
        extra_vertices: Vec<Vertex>,
        gui: &mut Gui,
    ) {
        let (width, height) = gpu::window_size(&self.gpu.window);
        let aspect = width / height;

        let instances = {
            let pos = camera_state.pos;
            let pos = point(pos.x, pos.y);
            let pos = pixel_to_hex(RENDER_LAYOUT, pos).round();

            // TODO move this constant
            const RANGE: TileUnit = 32;

            let o = Hex::new(RANGE, RANGE);

            let min = pos - o;
            let max = pos + o;

            let none = InstanceData::new().model(
                *self
                    .resource_man
                    .registry
                    .get_tile(self.resource_man.registry.none)
                    .unwrap()
                    .models
                    .get(0)
                    .unwrap(),
            );

            let mut instances = map_render_info.instances;

            for q in min.q()..max.q() {
                for r in min.r()..max.r() {
                    let pos = Hex::new(q, r);
                    let p = hex_to_pixel(RENDER_LAYOUT, pos);

                    instances.entry(pos.into()).or_insert_with(|| {
                        none.position_offset([p.x as Num, p.y as Num, FAR as Num])
                    });
                }
            }

            if camera_state.is_at_max_height() {
                if let Some(instance) = instances.get_mut(&camera_state.pointing_at) {
                    *instance = instance.color_offset(colors::ORANGE.with_alpha(0.5).to_array())
                }
            }

            let mut instances = instances.into_values().collect::<Vec<_>>();

            instances.sort_by_key(|v| v.id);

            instances
        };

        let camera_pos = camera_state.pos.cast::<Num>().unwrap();
        let pos = actual_pos(camera_pos, eye(camera_pos.z, PI));

        let matrix = matrix(camera_pos, aspect as Num, PI);

        /*
        let camera_pos = matrix * vec4(0.0, 0.0, 6.0, 1.0);
        println!("{:?}", camera_pos);
        let camera_pos = camera_pos.truncate() / camera_pos.w;
        let camera_pos = point3(-camera_pos.x, camera_pos.y, 1.0);
        println!("{:?}", camera_pos);
         */

        self.inner_render(matrix, pos, &instances, &gui_instances, extra_vertices, gui);
    }

    fn inner_render(
        &mut self,
        matrix: Matrix4,
        camera_pos: Point3,
        instances: &[InstanceData],
        gui_instances: &[InstanceData],
        _extra_vertices: Vec<Vertex>,
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
                Err(e) => panic!("failed to acquire next image: {e:?}"),
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

        {
            let indirect_instance =
                gpu::indirect_instance(&allocator, &self.resource_man, instances);
            let ubo = GameUBO::new(matrix, camera_pos);

            *self.gpu.alloc.game_uniform_buffer.write().unwrap() = ubo;

            let ubo_layout = self.gpu.game_pipeline.layout().set_layouts()[0].clone();

            let ubo_set = PersistentDescriptorSet::new(
                &self.gpu.alloc.descriptor_allocator,
                ubo_layout,
                [WriteDescriptorSet::buffer(
                    0,
                    self.gpu.alloc.game_uniform_buffer.clone(),
                )],
            )
            .unwrap();

            if let Some((indirect_commands, instance_buffer)) = indirect_instance {
                game_builder
                    .set_viewport(0, [gpu::viewport(&self.gpu.window)])
                    .bind_pipeline_graphics(self.gpu.game_pipeline.clone())
                    .bind_vertex_buffers(0, (self.gpu.alloc.vertex_buffer.clone(), instance_buffer))
                    .bind_index_buffer(self.gpu.alloc.index_buffer.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.gpu.game_pipeline.layout().clone(),
                        0,
                        ubo_set,
                    )
                    .draw_indexed_indirect(indirect_commands)
                    .unwrap();
            }
        }

        // game
        builder
            .execute_commands(game_builder.build().unwrap())
            .unwrap();

        builder
            .next_subpass(SubpassContents::SecondaryCommandBuffers)
            .unwrap();

        // extra gui
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

        {
            gui_builder
                .set_viewport(0, [gpu::viewport(&self.gpu.window)])
                .set_scissor(0, [Scissor::irrelevant()]);

            if !gui_instances.is_empty() {
                let ubo = GuiUBO {
                    matrix: matrix.into(),
                };

                *self.gpu.alloc.gui_uniform_buffer.write().unwrap() = ubo;

                let gui_ubo_set = PersistentDescriptorSet::new(
                    &self.gpu.alloc.descriptor_allocator,
                    self.gpu.gui_pipeline.layout().set_layouts()[0].clone(),
                    [WriteDescriptorSet::buffer(
                        0,
                        self.gpu.alloc.gui_uniform_buffer.clone(),
                    )],
                )
                .unwrap();

                if let Some((indirect_commands, instance_buffer)) =
                    gpu::indirect_instance(&allocator, &self.resource_man, gui_instances)
                {
                    gui_builder
                        .bind_pipeline_graphics(self.gpu.gui_pipeline.clone())
                        .bind_vertex_buffers(
                            0,
                            (self.gpu.alloc.vertex_buffer.clone(), instance_buffer),
                        )
                        .bind_index_buffer(self.gpu.alloc.index_buffer.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            self.gpu.gui_pipeline.layout().clone(),
                            0,
                            gui_ubo_set,
                        )
                        .draw_indexed_indirect(indirect_commands)
                        .unwrap();
                }
            }

            /*
            if !extra_vertices.is_empty() {
                let ubo = GuiUBO {
                    matrix: Matrix4::identity().into(),
                };

                *self.gpu.alloc.gui_uniform_buffer.write().unwrap() = ubo;

                let gui_ubo_set = PersistentDescriptorSet::new(
                    &self.gpu.alloc.descriptor_allocator,
                    self.gpu.gui_pipeline.layout().set_layouts()[0].clone(),
                    [WriteDescriptorSet::buffer(
                        0,
                        self.gpu.alloc.gui_uniform_buffer.clone(),
                    )],
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
                    .bind_pipeline_graphics(self.gpu.gui_pipeline.clone())
                    .bind_vertex_buffers(0, extra_vertex_buffer)
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.gpu.gui_pipeline.layout().clone(),
                        0,
                        gui_ubo_set,
                    )
                    .draw(vertex_count as u32, 1, 0, 0)
                    .unwrap();
            }
             */
        }

        if let Ok(commands) = gui_builder.build() {
            builder.execute_commands(commands).unwrap();
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
