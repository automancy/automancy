use std::sync::Arc;

use vulkano::{buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer}, command_buffer::DrawIndexedIndirectCommand, device::{
    Device, Queue,
}, image::{AttachmentImage, SwapchainImage, view::ImageView}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass}, swapchain::Surface, sync, sync::GpuFuture};
use vulkano::buffer::DeviceLocalBuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::format::Format;
use vulkano::image::{ImageAccess, ImageUsage, SampleCount};
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::swapchain::{Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo};
use vulkano::sync::FlushError;
use winit::{dpi::LogicalSize, window::Window};

use crate::render::data::Vertex;
use crate::util::cg::Double;
use crate::render::gpu;
use crate::util::cg::Num;
use crate::util::init::InitData;

use super::data::{InstanceData, UniformBufferObject};

pub mod vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "game/shaders/vert.glsl"
    }
}

pub mod frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "game/shaders/frag.glsl"
    }
}

pub mod gui_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "game/shaders/vert_gui.glsl"
    }
}

pub mod gui_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "game/shaders/frag_gui.glsl"
    }
}

pub fn immutable_buffer<T, D, W>(
    allocator: &(impl MemoryAllocator + ?Sized),
    data: D,
    buffer_usage: BufferUsage,
    builder: &mut AutoCommandBufferBuilder<W>
) -> Arc<DeviceLocalBuffer<[T]>>
    where
        D: IntoIterator<Item = T>,
        D::IntoIter: ExactSizeIterator,
        [T]: BufferContents
{
    let buffer = DeviceLocalBuffer::from_iter(allocator, data, buffer_usage, builder)
        .expect("failed to create vertex buffer");

    buffer
}

pub fn cpu_accessible_buffer<T, D>(
    allocator: &(impl MemoryAllocator + ?Sized),
    data: D,
    buffer_usage: BufferUsage,
) -> Arc<CpuAccessibleBuffer<[T]>>
where
    D: IntoIterator<Item = T>,
    D::IntoIter: ExactSizeIterator,
    [T]: BufferContents
{
    let buffer = CpuAccessibleBuffer::from_iter(allocator, buffer_usage, false, data)
        .expect("failed to create vertex buffer");

    buffer
}

pub fn uniform_buffer(
    allocator: &(impl MemoryAllocator + ?Sized)
) -> Arc<CpuAccessibleBuffer<UniformBufferObject>> {
    CpuAccessibleBuffer::from_data(
        allocator,
        BufferUsage {
            uniform_buffer: true,
            ..Default::default()
        },
        false,
        UniformBufferObject::default(),
    ).unwrap()
}

pub fn framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    color_image: Arc<AttachmentImage>,
    depth_buffer: Arc<AttachmentImage>,
    depth_buffer_egui: Arc<AttachmentImage>,
) -> Vec<Arc<Framebuffer>> {
    let color_image_view = ImageView::new_default(color_image).unwrap();
    let depth_buffer = ImageView::new_default(depth_buffer).unwrap();
    let depth_buffer_egui = ImageView::new_default(depth_buffer_egui).unwrap();

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, color_image_view.clone(), depth_buffer.clone(), depth_buffer_egui.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

fn get_window_size(window: &Window) -> LogicalSize<u32> {
    window.inner_size().to_logical(1.0) // ?
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

    viewport_with_dims([width as Num, height as Num])
}

pub fn viewport_with_dims(dimensions: [Num; 2]) -> Viewport {
    Viewport {
        origin: [0.0, 0.0],
        dimensions,
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

pub fn indirect_buffer(
    allocator: &(impl MemoryAllocator + ?Sized),
    init_data: &InitData,
    instances: &[InstanceData],
) -> (
    Arc<CpuAccessibleBuffer<[InstanceData]>>,
    Vec<DrawIndexedIndirectCommand>,
) {
    let indirect_commands = instances
        .group_by(|a, b| a.faces_index == b.faces_index)
        .scan(0, |init, instances| {
            let instance_count = instances.len() as u32;

            let first_instance = *init;

            let r = if let Some(faces) = &init_data.all_faces[instances[0].faces_index] {
                let commands = faces
                    .iter()
                    .map(|face| {
                        DrawIndexedIndirectCommand {
                            index_count: face.indices.len() as u32,
                            instance_count,
                            first_index: face.offset.unwrap(),
                            vertex_offset: 0,
                            first_instance,
                        }
                    })
                    .collect::<Vec<_>>();

                Some(commands)
            } else {
                None
            };

            *init += instance_count;

            r
        })
        .flatten()
        .collect::<Vec<_>>();

    let instance_buffer = cpu_accessible_buffer(
        allocator,
        instances.to_vec(),
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        }
    );

    (instance_buffer, indirect_commands)
}

impl Gpu {
    pub fn resize_image_with_samples(&self, sample_count: SampleCount, image: &mut Arc<AttachmentImage>, allocator: &(impl MemoryAllocator + ?Sized), size: [u32; 2], recreate_swapchain: &mut bool) {
        if size != image.dimensions().width_height() {
            *recreate_swapchain = true;

            *image = AttachmentImage::multisampled_with_usage(
                allocator,
                size,
                sample_count,
                image.format(),
                *image.usage(),
            ).unwrap();
        }
    }

    pub fn resize_image(&self, image: &mut Arc<AttachmentImage>, allocator: &(impl MemoryAllocator + ?Sized), size: [u32; 2], recreate_swapchain: &mut bool) {
        if size != image.dimensions().width_height() {
            *recreate_swapchain = true;

            *image = AttachmentImage::with_usage(
                allocator,
                size,
                image.format(),
                *image.usage(),
            ).unwrap();
        }
    }

    pub fn recreate_swapchain(
        &self,
        size: [u32; 2],
        color_image: Arc<AttachmentImage>,
        depth_buffer: Arc<AttachmentImage>,
        depth_buffer_egui: Arc<AttachmentImage>,
        swapchain: &mut Arc<Swapchain>,
        framebuffers: &mut Vec<Arc<Framebuffer>>,
        recreate_swapchain: &mut bool
    ) {
        let (new_swapchain, new_images) = {
            match swapchain.recreate(SwapchainCreateInfo {
                image_extent: size,
                ..swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("failed to recreate swapchain: {:?}", e),
            }
        };

        *swapchain = new_swapchain;
        *framebuffers = gpu::framebuffers(&new_images, self.render_pass.clone(), color_image.clone(), depth_buffer.clone(), depth_buffer_egui.clone());
        *recreate_swapchain = false;
    }

    pub fn commit_commands(
        &self,
        image_num: usize,
        swapchain: Arc<Swapchain>,
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
            .then_swapchain_present(self.queue.clone(), SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_num as u32))
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

pub fn indirect_instance(
    allocator: &(impl MemoryAllocator + ?Sized),
    init_data: &InitData,
    instances: &[InstanceData]
) -> Option<(Arc<CpuAccessibleBuffer<[DrawIndexedIndirectCommand]>>, Arc<CpuAccessibleBuffer<[InstanceData]>>)> {
    if instances.is_empty() {
        None
    } else {
        let (instance_buffer, commands) = indirect_buffer(
            allocator, init_data, instances
        );

        if commands.is_empty() {
            None
        } else {
            let indirect_buffer = cpu_accessible_buffer(
                allocator,
                commands.into_iter(),
                BufferUsage {
                    indirect_buffer: true,
                    ..Default::default()
                }
            );
            Some((indirect_buffer, instance_buffer))
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

    pub vertex_buffer: Arc<DeviceLocalBuffer<[Vertex]>>,
    pub index_buffer: Arc<DeviceLocalBuffer<[u32]>>,
    pub uniform_buffer: Arc<CpuAccessibleBuffer<UniformBufferObject>>,
}
