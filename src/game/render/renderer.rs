use std::collections::HashMap;
use std::ops::{Add, Div, Sub};
use std::sync::Arc;
use cgmath::{EuclideanSpace, point2, point3, SquareMatrix, Transform, vec2};
use hexagon_tiles::hexagon::{Hex, HexMath, HexRound, OffsetCoord};
use hexagon_tiles::layout::{Layout, LayoutTool};
use hexagon_tiles::point::Point;
use hexagon_tiles::tools::{HEX_ODD, HexOffset};

use riker::actors::{ActorRef, ActorSystem};
use riker_patterns::ask::ask;
use serde_json::to_vec;

use vulkano::buffer::{BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::format::{ClearValue};
use vulkano::memory::allocator::{FastMemoryAllocator};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::swapchain::{acquire_next_image, AcquireError};
use vulkano::sync;
use vulkano::sync::GpuFuture;

use crate::game::data::map::{MapRenderInfo, RenderContext};
use crate::game::data::tile;
use crate::game::data::tile::{TileCoord, TileUnit};
use crate::game::game::GameMsg;
use crate::game::render::camera::{CameraState};
use crate::game::render::data::{FAR, InstanceData, RENDER_LAYOUT, UniformBufferObject};
use crate::game::render::gpu;
use crate::game::render::gpu::{Gpu, window_size_u32};
use crate::math::cg::{Matrix4, Num, Point3, Vector2};
use crate::registry::init::InitData;




pub struct Renderer {
    game: ActorRef<GameMsg>,

    init_data: Arc<InitData>,

    gpu: Gpu,
    previous_frame_end: Option<Box<dyn GpuFuture + Send + Sync>>,
    pub recreate_swapchain: bool,

    instances: Vec<InstanceData>,
}

impl Renderer {
    pub fn new(
        game: ActorRef<GameMsg>,

        init_data: Arc<InitData>,

        gpu: Gpu,
    ) -> Self {
        let device = gpu.device.clone();

        Self {
            game,

            init_data,

            gpu,
            previous_frame_end: Some(sync::now(device).boxed_send_sync()),
            recreate_swapchain: false,

            instances: vec![],
        }
    }
}


impl Renderer {
    pub async fn render(&mut self, sys: &ActorSystem, camera_state: CameraState) {
        let (width, height) = gpu::window_size(&self.gpu.window);
        let aspect = width / height;

        let camera_pos = camera_state.pos;

        self.instances = {
            let pos = Point { x: camera_pos.x as f64, y: camera_pos.y as f64 };
            let pos = LayoutTool::pixel_to_hex(RENDER_LAYOUT, pos).round();

            // TODO move this constant
            const RANGE: TileUnit = 32;

            let o = Hex::new(RANGE, RANGE);

            let min = pos.sub(o);
            let max = pos.add(o);

            let none = InstanceData::new().faces_index(
                self.init_data.resource_man.resources[&tile::NONE]
                    .faces_index
                    .unwrap(),
            );

            let map_render_info: MapRenderInfo = ask(sys, &self.game, GameMsg::RenderInfoRequest {
                context: RenderContext {
                    init_data: self.init_data.clone(),
                }
            }).await;

            let mut instances = map_render_info.instances;

            for q in min.q()..max.q() {
                for r in min.r()..max.r() {
                    let pos = Hex::new(q, r);
                    let p = LayoutTool::hex_to_pixel(RENDER_LAYOUT, pos);

                    instances.entry(pos).or_insert_with(|| none.position_offset([p.x as Num, p.y as Num, FAR]));
                }
            }

            if 1.0 - camera_pos.z < 0.99 {
                let size = vec2(width as f64, height as f64) / 2.0;

                let c = camera_state.main_pos.to_vec();
                let c = c.cast::<f64>().unwrap();
                let c = c.zip(size, Sub::sub);
                let c = c.zip(size, Div::div);
                let c = point3(c.x, c.y, FAR as f64);

                let matrix = CameraState::matrix(point3(0.0, 0.0, camera_state.pos.z), aspect).cast::<f64>().unwrap();

                let v = c.to_vec();
                let v = matrix * v.extend(1.0);
                let v = v.truncate().truncate() * v.w;

                let pointing = Point { x: v.x, y: v.y };
                let pointing = LayoutTool::pixel_to_hex(RENDER_LAYOUT, pointing).round();
                let pointing = pointing.add(pos);

                instances.get_mut(&pointing).map(|instance| {
                    *instance = instance
                        .add_position_offset([0.0, 0.0, 0.001])
                        .color_offset([1.0, 0.745, 0.447, 0.5])
                });
            }

            let mut instances = instances.into_values().collect::<Vec<_>>();

            instances.sort_by_key(|v| v.faces_index);

            instances
        };

        self.inner_render(CameraState::matrix(camera_pos, aspect))
    }

    fn inner_render(&mut self, matrix: Matrix4) {
        let size = window_size_u32(&self.gpu.window);

        if size[0] == 0 || size[1] == 0 {
            return;
        }

        let allocator = FastMemoryAllocator::new_default(self.gpu.device.clone());

        self.gpu.depth_buffer_size(&allocator, size, &mut self.recreate_swapchain);

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            self.gpu.recreate_swapchain(size, &mut self.recreate_swapchain)
        }

        let (image_num, suboptimal, acquire_future) = {
            match acquire_next_image(self.gpu.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {:?}", e),
            }
        };

        let image_num = image_num as usize;

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let command_allocator = StandardCommandBufferAllocator::new(
            self.gpu.device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                ..Default::default()
            }
        );

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            &command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        // TODO improve this garbage
        let instances = self.instances.as_slice();

        let indirect_instance = if !instances.is_empty() {
            let (instance_buffer, commands) = gpu::indirect_buffer(
                &allocator, &self.init_data, instances, &mut command_buffer_builder
            );

            if !commands.is_empty() {
                let indirect_buffer = gpu::immutable_buffer(
                    &allocator,
                    commands.into_iter(),
                    BufferUsage {
                        indirect_buffer: true,
                        ..Default::default()
                    },
                    &mut command_buffer_builder,
                );
                Some((indirect_buffer, instance_buffer))
            } else {
                None
            }
        } else {
            None
        };

        let ubo = UniformBufferObject {
            matrix: matrix.into(),
        };

        *self.gpu.uniform_buffers[image_num].write().unwrap() = ubo;

        let pipeline_layout = self.gpu.pipeline.layout();
        let ubo_layout = pipeline_layout.set_layouts()[0].clone();

        let descriptor_allocator = StandardDescriptorSetAllocator::new(self.gpu.device.clone());

        let ubo_set = PersistentDescriptorSet::new(
            &descriptor_allocator,
            ubo_layout,
            [WriteDescriptorSet::buffer(
                0,
                self.gpu.uniform_buffers[image_num].clone(),
            )],
        ).unwrap();

        /*
        let debug_pipeline_layout = debug_pipeline.layout();
        let debug_ubo_layout = debug_pipeline_layout.set_layouts()[0].clone();
        let debug_ubo_set = PersistentDescriptorSet::new(
            debug_ubo_layout.clone(),
            [WriteDescriptorSet::buffer(
                0,
                uniform_buffers[image_num].clone(),
            )],
        )
        .unwrap();
         */

        let mut render_pass_info = RenderPassBeginInfo::framebuffer(self.gpu.framebuffers[image_num].clone());
        render_pass_info.render_pass = self.gpu.render_pass.clone();
        render_pass_info.clear_values = vec![
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::DepthStencil((1.0, 0))),
        ];

        command_buffer_builder
            .begin_render_pass(
                render_pass_info,
                SubpassContents::Inline
            )
            .unwrap()
            .set_viewport(0, [gpu::viewport(&self.gpu.window)]);

        if let Some((indirect_buffer, instance_buffer)) = indirect_instance {
            command_buffer_builder
                .bind_vertex_buffers(0, (self.gpu.vertex_buffer.clone(), instance_buffer))
                .bind_index_buffer(self.gpu.index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline_layout.clone(),
                    0,
                    ubo_set,
                )
                .bind_pipeline_graphics(self.gpu.pipeline.clone())
                .draw_indexed_indirect(indirect_buffer)
                .unwrap();
        }

        /*
        let debug_vertex_buffer_size = debug_vertex_buffer.len() as u32;
        if debug_vertex_buffer_size != 0 {
            builder
                .bind_vertex_buffers(0, debug_vertex_buffer)
                .bind_pipeline_graphics(debug_pipeline)
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    debug_pipeline_layout.clone(),
                    0,
                    debug_ubo_set,
                )
                .draw(debug_vertex_buffer_size, 1, 0, 0)
                .unwrap();
        }
         */

        command_buffer_builder.end_render_pass().unwrap();

        let command_buffer = command_buffer_builder.build().unwrap();

        self.gpu.commit_commands(image_num, acquire_future, command_buffer, &mut self.previous_frame_end, &mut self.recreate_swapchain);
    }
}
