use std::sync::Arc;

use vulkano::{buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer}, command_buffer::DrawIndexedIndirectCommand, device::{
    Device, Queue,
}, image::{view::ImageView, AttachmentImage, SwapchainImage}, pipeline::{
    graphics::{
        depth_stencil::DepthStencilState,
        input_assembly::{InputAssemblyState, PrimitiveTopology},
        rasterization::RasterizationState,
        vertex_input::BuffersDefinition,
        viewport::{Viewport, ViewportState},
    },
    GraphicsPipeline,
}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, shader::ShaderModule, swapchain::Surface, sync, sync::GpuFuture};
use vulkano::buffer::DeviceLocalBuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::format::Format;
use vulkano::image::{ImageAccess, ImageUsage};
use vulkano::memory::allocator::{MemoryAllocator};
use vulkano::swapchain::{Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo};
use vulkano::sync::{FlushError};
use winit::{dpi::LogicalSize, window::Window};

use crate::{game::render::data::Vertex, math::cg::Num, registry::init::InitData};
use crate::math::cg::{Double};

use super::data::{InstanceData, UniformBufferObject};

pub mod vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/vert.glsl"
    }
}

pub mod dbg_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/dbg_vert.glsl"
    }
}

pub mod frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/frag.glsl"
    }
}

pub mod dbg_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/dbg_frag.glsl"
    }
}

pub fn pipeline(
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    topology: PrimitiveTopology,
    rasterization: RasterizationState,
    instanced: bool,
) -> Arc<GraphicsPipeline> {
    let mut vertex_input_state = BuffersDefinition::new().vertex::<Vertex>();

    if instanced {
        vertex_input_state = vertex_input_state.instance::<InstanceData>();
    }

    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(vertex_input_state)
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new().topology(topology))
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .rasterization_state(rasterization)
        .depth_stencil_state(DepthStencilState::simple_depth_test());

    pipeline.build(device.clone()).unwrap()
}

pub fn immutable_buffer<T, D, W>(
    allocator: &(impl MemoryAllocator + ?Sized),
    data: D,
    buffer_usage: BufferUsage,
    command_buffer_builder: &mut AutoCommandBufferBuilder<W>
) -> Arc<DeviceLocalBuffer<[T]>>
where
    D: IntoIterator<Item = T>,
    D::IntoIter: ExactSizeIterator,
    [T]: BufferContents
{
    let buffer = DeviceLocalBuffer::from_iter(allocator, data, buffer_usage, command_buffer_builder)
        .expect("failed to create vertex buffer");

    buffer
}

pub fn uniform_buffers(
    allocator: &(impl MemoryAllocator + ?Sized),
    image_count: u32,
) -> Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>> {
    (0..image_count)
        .map(|_| {
            CpuAccessibleBuffer::from_data(
                allocator,
                BufferUsage {
                    uniform_buffer: true,
                    ..Default::default()
                },
                false,
                UniformBufferObject::default(),
            )
            .unwrap()
        })
        .collect()
}

pub fn framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    depth_buffer: Arc<AttachmentImage>,
) -> Vec<Arc<Framebuffer>> {
    let depth_buffer = ImageView::new_default(depth_buffer).unwrap();

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_buffer.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

fn get_window_size(window: &Window) -> LogicalSize<u32> {
    window.inner_size().to_logical(window.scale_factor())
}

pub fn window_size(window: &Window) -> (Double, Double) {
    get_window_size(window).cast::<Double>().into()
}

pub fn window_size_u32(window: &Window) -> [u32; 2] {
    let size = get_window_size(window);

    [size.width, size.height]
}

pub fn viewport(window: &Window) -> Viewport {
    let (width, height) = window_size(window);

    Viewport {
        origin: [0.0, 0.0],
        dimensions: [width as Num, height as Num],
        depth_range: 0.0..1.0,
    }
}

pub fn command_buffer_builder(device: Arc<Device>, queue: Arc<Queue>) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    let command_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo {
            ..Default::default()
        }
    );

    AutoCommandBufferBuilder::primary(
        &command_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    ).unwrap()
}

pub fn indirect_buffer<W>(
    allocator: &(impl MemoryAllocator + ?Sized),
    init_data: &InitData,
    instances: &[InstanceData],
    command_buffer_builder: &mut AutoCommandBufferBuilder<W>
) -> (
    Arc<DeviceLocalBuffer<[InstanceData]>>,
    Vec<DrawIndexedIndirectCommand>,
) {
    let indirect_commands = instances
        .group_by(|a, b| a.faces_index == b.faces_index)
        .scan(0, |init, instances| {
            let len = instances.len() as u32;

            let first_instance = *init;
            *init += len;

            if let Some(faces) = &init_data.all_faces[instances[0].faces_index] {
                let commands = faces
                    .iter()
                    .scan(0, |init, face| {
                        let face_len = face.vertex_indices.len() as u32;

                        *init += face_len;

                        Some(DrawIndexedIndirectCommand {
                            first_index: *init,
                            index_count: face_len,
                            first_instance,
                            instance_count: len,
                            vertex_offset: 0,
                        })
                    })
                    .collect::<Vec<_>>();

                Some(commands)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    let instance_buffer = immutable_buffer(
        allocator,
        instances.to_vec(),
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        },
        command_buffer_builder
    );

    (instance_buffer, indirect_commands)
}

impl Gpu {
    pub fn depth_buffer_size(&mut self, allocator: &(impl MemoryAllocator + ?Sized), size: [u32; 2], recreate_swapchain: &mut bool) {
        if size != ImageAccess::dimensions(&self.depth_buffer).width_height() {
            *recreate_swapchain = true;

            self.depth_buffer = AttachmentImage::with_usage(
                allocator,
                size,
                Format::D24_UNORM_S8_UINT,
                ImageUsage {
                    depth_stencil_attachment: true,
                    ..Default::default()
                },
            ).unwrap();
        }
    }

    pub fn recreate_swapchain(&mut self, size: [u32; 2], recreate_swapchain: &mut bool) {
        let (new_swapchain, new_images) = {
            match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: size,
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("failed to recreate swapchain: {:?}", e),
            }
        };

        self.swapchain = new_swapchain;

        self.framebuffers = framebuffers(&new_images, self.render_pass.clone(), self.depth_buffer.clone());
        *recreate_swapchain = false;
    }

    pub fn commit_commands(
        &mut self,
        image_num: usize,
        acquire_future: SwapchainAcquireFuture,
        command_buffer: PrimaryAutoCommandBuffer,
        previous_frame_end: &mut Option<Box<dyn GpuFuture + Send + Sync>>,
        recreate_swapchain: &mut bool
    ) {
        let future = previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_num as u32))
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                future.wait(None).unwrap();
                *previous_frame_end = Some(future.boxed_send_sync());
            }
            Err(FlushError::OutOfDate) => {
                *recreate_swapchain = true;
                *previous_frame_end = Some(sync::now(self.device.clone()).boxed_send_sync());
            }
            Err(e) => {
                log::error!("failed to flush future: {:?}", e);
                *previous_frame_end = Some(sync::now(self.device.clone()).boxed_send_sync());
            }
        }
    }
}

#[derive(Clone)]
pub struct Gpu {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface>,
    pub render_pass: Arc<RenderPass>,
    pub window: Arc<Window>,

    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,

    pub vertex_buffer: Arc<DeviceLocalBuffer<[Vertex]>>,
    pub index_buffer: Arc<DeviceLocalBuffer<[u32]>>,
    pub depth_buffer: Arc<AttachmentImage>,
    pub uniform_buffers: Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>>,

    pub pipeline: Arc<GraphicsPipeline>,
}
