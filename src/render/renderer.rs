use std::f32::consts::PI;
use std::sync::Arc;
use egui_winit_vulkano::Gui;
use hexagon_tiles::hex::Hex;
use hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};

use hexagon_tiles::point::{point};
use hexagon_tiles::traits::HexRound;

use riker::actors::{ActorRef, ActorSystem};
use riker_patterns::ask::ask;

use vulkano::buffer::{BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::format::{ClearValue};
use vulkano::image::AttachmentImage;
use vulkano::memory::allocator::{FastMemoryAllocator};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, Subpass};
use vulkano::swapchain::{acquire_next_image, AcquireError, Swapchain};
use vulkano::sync;
use vulkano::sync::GpuFuture;

use crate::data::map::{MapRenderInfo, RenderContext};
use crate::data::tile;
use crate::data::tile::{TileCoord, TileUnit};
use crate::game::game::GameMsg;
use crate::render::camera::{CameraState, FAR};
use crate::render::data::{InstanceData, RENDER_LAYOUT, UniformBufferObject};
use crate::render::gpu;
use crate::math::cg::{matrix, Matrix4, Num};
use crate::registry::init::InitData;
use crate::render::gpu::Gpu;
use crate::util::colors::Color;

pub struct Renderer {
    game: ActorRef<GameMsg>,

    init_data: Arc<InitData>,

    pub recreate_swapchain: bool,

    gpu: Arc<Gpu>,
    pipeline: Arc<GraphicsPipeline>,
    depth_buffer: Arc<AttachmentImage>,
    depth_buffer_gui: Arc<AttachmentImage>,
    swapchain: Arc<Swapchain>,
    framebuffers: Vec<Arc<Framebuffer>>,
    previous_frame_end: Option<Box<dyn GpuFuture + Send + Sync>>,
}

impl Renderer {
    pub fn new(
        game: ActorRef<GameMsg>,

        init_data: Arc<InitData>,

        gpu: Arc<Gpu>,
        pipeline: Arc<GraphicsPipeline>,
        depth_buffer: Arc<AttachmentImage>,
        depth_buffer_gui: Arc<AttachmentImage>,
        swapchain: Arc<Swapchain>,
        framebuffers: Vec<Arc<Framebuffer>>,
    ) -> Self {
        let device = gpu.device.clone();

        Self {
            game,

            init_data,

            recreate_swapchain: false,

            gpu,
            pipeline,
            depth_buffer,
            depth_buffer_gui,
            swapchain,
            framebuffers,
            previous_frame_end: Some(sync::now(device).boxed_send_sync()),
        }
    }
}


impl Renderer {
    pub async fn render(
        &mut self,
        sys: &ActorSystem,
        camera_state: CameraState,
        subpass: Subpass,
        gui: &mut Gui
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
                    let p = hex_to_pixel(RENDER_LAYOUT, pos);

                    instances.entry(TileCoord(pos)).or_insert_with(|| none.position_offset([p.x as Num, p.y as Num, FAR as Num]));
                }
            }

            if camera_pos.z > 0.98 {
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

            instances
        };

        self.inner_render(matrix(camera_pos.cast::<Num>().unwrap(), aspect as Num, PI), subpass, &instances, gui)
    }

    fn inner_render(&mut self, matrix: Matrix4, subpass: Subpass, instances: &[InstanceData], gui: &mut Gui) {
        let dimensions = gpu::window_size_u32(&self.gpu.window);

        if dimensions[0] == 0 || dimensions[1] == 0 {
            return;
        }

        let allocator = FastMemoryAllocator::new_default(self.gpu.device.clone());

        self.gpu.depth_buffer_size(&mut self.depth_buffer, &allocator, dimensions, &mut self.recreate_swapchain);
        self.gpu.depth_buffer_size(&mut self.depth_buffer_gui, &allocator, dimensions, &mut self.recreate_swapchain);

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            self.gpu.recreate_swapchain(dimensions, self.depth_buffer.clone(), self.depth_buffer_gui.clone(), &mut self.swapchain, &mut self.framebuffers, &mut self.recreate_swapchain);
        }

        let (image_num, suboptimal, acquire_future) = {
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {:?}", e),
            }
        };
        if suboptimal { self.recreate_swapchain = true; }
        let image_num = image_num as usize;

        let command_allocator = StandardCommandBufferAllocator::new(
            self.gpu.device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                ..Default::default()
            }
        );

        let mut builder = AutoCommandBufferBuilder::primary(
            &command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::DepthStencil((1.0, 0))),
                        Some(ClearValue::DepthStencil((1.0, 0))),
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[image_num].clone())
                },
                SubpassContents::SecondaryCommandBuffers
            ).unwrap();


        let mut secondary_builder = AutoCommandBufferBuilder::secondary(
            &command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(subpass.clone().into()),
                ..Default::default()
            },
        ).unwrap();

        secondary_builder
            .set_viewport(0, [gpu::viewport(&self.gpu.window)]);

        let indirect_instance = gpu::indirect_instance(&allocator, &self.init_data, instances);

        let ubo = UniformBufferObject {
            matrix: matrix.into(),
        };

        *self.gpu.uniform_buffer.write().unwrap() = ubo;

        let ubo_layout = self.pipeline.layout().set_layouts()[0].clone();

        let descriptor_allocator = StandardDescriptorSetAllocator::new(self.gpu.device.clone());

        let ubo_set = PersistentDescriptorSet::new(
            &descriptor_allocator,
            ubo_layout,
            [WriteDescriptorSet::buffer(
                0,
                self.gpu.uniform_buffer.clone(),
            )],
        ).unwrap();

        if let Some((indirect_buffer, instance_buffer)) = indirect_instance {
            secondary_builder
                .bind_pipeline_graphics(self.pipeline.clone())
                .bind_vertex_buffers(0, (self.gpu.vertex_buffer.clone(), instance_buffer))
                .bind_index_buffer(self.gpu.index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    ubo_set,
                )
                .draw_indexed_indirect(indirect_buffer)
                .unwrap();
        }

        // game
        let secondary_buffer = secondary_builder.build().unwrap();
        builder.execute_commands(secondary_buffer).unwrap();


        // gui
        builder.next_subpass(SubpassContents::SecondaryCommandBuffers).unwrap();
        let gui_command_buffer = gui.draw_on_subpass_image(dimensions);
        builder.execute_commands(gui_command_buffer).unwrap();


        // end
        builder.end_render_pass().unwrap();


        let command_buffer = builder.build().unwrap();
        self.gpu.commit_commands(image_num, self.swapchain.clone(), acquire_future, command_buffer, &mut self.previous_frame_end, &mut self.recreate_swapchain);
    }
}
