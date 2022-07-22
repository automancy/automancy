use std::{collections::BTreeMap, sync::Arc};

use vulkano::{
    buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer, ImmutableBuffer},
    command_buffer::DrawIndexedIndirectCommand,
    descriptor_set::layout::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
        DescriptorType,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily},
        Device, DeviceExtensions, Queue,
    },
    image::{view::ImageView, SwapchainImage},
    instance::Instance,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        layout::PipelineLayoutCreateInfo,
        GraphicsPipeline, PipelineLayout,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::Surface,
    sync::GpuFuture,
};
use winit::window::Window;

use crate::{game::render::data::Vertex, registry::init::InitData};

use super::data::{InstanceData, UniformBufferObject};

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/vert.glsl"
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/frag.glsl"
    }
}

pub fn select_physical_device<'a>(
    instance: &'a Arc<Instance>,
    surface: Arc<Surface<Window>>,
    device_extensions: &DeviceExtensions,
) -> (PhysicalDevice<'a>, QueueFamily<'a>) {
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|q| {
                    q.supports_compute()
                        && q.supports_graphics()
                        && q.supports_surface(&surface).unwrap_or(false)
                })
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .expect("no device available");

    (physical_device, queue_family)
}

pub fn pipeline(
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
) -> Arc<GraphicsPipeline> {
    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(
            BuffersDefinition::new()
                .vertex::<Vertex>()
                .instance::<InstanceData>(),
        )
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .multisample_state(MultisampleState {
            sample_shading: Some(1.0),
            ..Default::default()
        });

    pipeline.build(device.clone()).unwrap()
}

pub fn immutable_buffer<T, D>(
    data: D,
    buffer_usage: BufferUsage,
    queue: Arc<Queue>,
) -> Arc<ImmutableBuffer<[T]>>
where
    D: IntoIterator<Item = T>,
    D::IntoIter: ExactSizeIterator,
    [T]: BufferContents,
{
    let (buffer, future) = ImmutableBuffer::from_iter(data, buffer_usage, queue.clone())
        .expect("failed to create vertex buffer");
    future.flush().unwrap();

    buffer
}

pub fn uniform_buffers(
    device: &Arc<Device>,
    image_count: u32,
) -> Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>> {
    (0..image_count)
        .map(|_| {
            CpuAccessibleBuffer::from_data(
                device.clone(),
                BufferUsage::uniform_buffer(),
                false,
                UniformBufferObject::default(),
            )
            .unwrap()
        })
        .collect()
}

pub fn framebuffers(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

pub fn viewport() -> Viewport {
    Viewport {
        origin: [0.0, 0.0],
        dimensions: [1024.0, 1024.0],
        depth_range: 0.0..1.0,
    }
}

pub fn indirect_buffer(
    queue: Arc<Queue>,
    init_data: &InitData,
    instance_buffer: &[InstanceData],
) -> (
    Arc<ImmutableBuffer<[InstanceData]>>,
    Vec<DrawIndexedIndirectCommand>,
) {
    let mut instances = instance_buffer.to_vec();
    instances.sort_by_key(|v| v.faces_index);

    let mut indirect_commands: Vec<Vec<_>> = Vec::with_capacity(instances.len()); // minimum size
    instances
        .group_by(|a, b| a.faces_index == b.faces_index)
        .fold(0, |first_instance, instances| {
            let len = instances.len() as u32;

            let index_ranges = &init_data.all_index_ranges[instances[0].faces_index];
            let commands = index_ranges
                .iter()
                .map(|range| DrawIndexedIndirectCommand {
                    first_index: range.start,
                    index_count: range.end - range.start,
                    first_instance,
                    instance_count: len,
                    vertex_offset: 0,
                })
                .collect();

            indirect_commands.push(commands);
            first_instance + len
        });

    let indirect_commands = indirect_commands.into_iter().flatten().collect::<Vec<_>>();

    let instance_buffer = immutable_buffer(
        instances.into_iter(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    );

    (instance_buffer, indirect_commands)
}
