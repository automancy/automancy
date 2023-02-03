use std::sync::Arc;

use bytemuck::Pod;
use futures_executor::block_on;
use vulkano::buffer::DeviceLocalBuffer;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
    PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{DeviceCreateInfo, DeviceExtensions, Features, QueueCreateInfo};
use vulkano::format::{Format, NumericType};
use vulkano::image::SampleCount::Sample4;
use vulkano::image::{ImageAccess, ImageUsage, SampleCount};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{MemoryAllocator, StandardMemoryAllocator};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::Subpass;
use vulkano::swapchain::{
    Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use vulkano::sync::FlushError;
use vulkano::{
    buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer},
    command_buffer::DrawIndexedIndirectCommand,
    device::{Device, Queue},
    image::{view::ImageView, AttachmentImage, SwapchainImage},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::Surface,
    sync,
    sync::GpuFuture,
    VulkanLibrary,
};
use vulkano_win::create_surface_from_winit;
use winit::event_loop::EventLoop;
use winit::window::{Icon, WindowBuilder};
use winit::{dpi::LogicalSize, window::Window};

use crate::render::data::{GuiUBO, Vertex};
use crate::util::cg::Double;
use crate::util::cg::Num;
use crate::util::resource::ResourceManager;

use super::data::{GameUBO, InstanceData};

pub mod vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "compile/shaders/vert.glsl"
    }
}

pub mod frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "compile/shaders/frag.glsl"
    }
}

pub mod gui_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "compile/shaders/vert_gui.glsl"
    }
}

pub mod gui_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "compile/shaders/frag_gui.glsl"
    }
}

pub fn create_game_pipeline(device: Arc<Device>, subpass: &Subpass) -> Arc<GraphicsPipeline> {
    let vs = vert_shader::load(device.clone()).unwrap();
    let fs = frag_shader::load(device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .vertex_input_state(
            BuffersDefinition::new()
                .vertex::<Vertex>()
                .instance::<InstanceData>(),
        )
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .rasterization_state(RasterizationState::new())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .multisample_state(MultisampleState {
            rasterization_samples: Sample4,
            ..Default::default()
        })
        .render_pass(subpass.clone());

    pipeline.build(device).unwrap()
}

pub fn create_gui_pipeline(device: Arc<Device>, subpass: &Subpass) -> Arc<GraphicsPipeline> {
    let vs_gui = gui_vert_shader::load(device.clone()).unwrap();
    let fs_gui = gui_frag_shader::load(device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_shader(vs_gui.entry_point("main").unwrap(), ())
        .vertex_input_state(
            BuffersDefinition::new()
                .vertex::<Vertex>()
                .instance::<InstanceData>(),
        )
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .fragment_shader(fs_gui.entry_point("main").unwrap(), ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
        .rasterization_state(RasterizationState::new())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .multisample_state(MultisampleState {
            rasterization_samples: Sample4,
            ..Default::default()
        })
        .render_pass(subpass.clone());

    pipeline.build(device).unwrap()
}

pub fn create_instance() -> Arc<Instance> {
    let library = VulkanLibrary::new().expect("no local Vulkan library");
    let required_extensions = vulkano_win::required_extensions(&library);

    Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            enumerate_portability: true,
            ..Default::default()
        },
    )
    .expect("failed to create instance")
}

pub fn create_window(icon: Icon, event_loop: &EventLoop<()>) -> Arc<Window> {
    Arc::new(
        WindowBuilder::new()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .build(event_loop)
            .expect("could not build window"),
    )
}

pub fn create_surface(window: Arc<Window>, instance: Arc<Instance>) -> Arc<Surface> {
    create_surface_from_winit(window, instance).expect("could not create surface")
}

pub fn get_physical_device(
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.graphics && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no devices available")
}

pub fn get_logical_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    device_extensions: DeviceExtensions,
) -> (Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
    Device::new(
        physical_device,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions,
            enabled_features: Features {
                multi_draw_indirect: true,
                fill_mode_non_solid: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("failed to create device")
}

fn get_window_size(window: &Window) -> LogicalSize<u32> {
    window.inner_size().to_logical(1.0)
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

pub fn immutable_buffer<T, D, W>(
    allocator: &(impl MemoryAllocator + ?Sized),
    data: D,
    buffer_usage: BufferUsage,
    builder: &mut AutoCommandBufferBuilder<W>,
) -> Arc<DeviceLocalBuffer<[T]>>
where
    D: IntoIterator<Item = T>,
    D::IntoIter: ExactSizeIterator,
    [T]: BufferContents,
{
    DeviceLocalBuffer::from_iter(allocator, data, buffer_usage, builder)
        .expect("failed to create vertex buffer")
}

pub fn cpu_accessible_buffer<T, D>(
    allocator: &(impl MemoryAllocator + ?Sized),
    data: D,
    buffer_usage: BufferUsage,
) -> Arc<CpuAccessibleBuffer<[T]>>
where
    D: IntoIterator<Item = T>,
    D::IntoIter: ExactSizeIterator,
    [T]: BufferContents,
{
    CpuAccessibleBuffer::from_iter(allocator, buffer_usage, false, data)
        .expect("failed to create vertex buffer")
}

pub fn uniform_buffer<UBO: Default + Pod + Send + Sync>(
    allocator: &(impl MemoryAllocator + ?Sized),
) -> Arc<CpuAccessibleBuffer<UBO>> {
    CpuAccessibleBuffer::from_data(
        allocator,
        BufferUsage {
            uniform_buffer: true,
            ..Default::default()
        },
        false,
        UBO::default(),
    )
    .unwrap()
}

pub fn framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    color_image: Arc<AttachmentImage>,
    depth_buffer: Arc<AttachmentImage>,
    depth_buffer_gui: Arc<AttachmentImage>,
) -> Vec<Arc<Framebuffer>> {
    let color_image_view = ImageView::new_default(color_image).unwrap();
    let depth_buffer = ImageView::new_default(depth_buffer).unwrap();
    let depth_buffer_gui = ImageView::new_default(depth_buffer_gui).unwrap();

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        color_image_view.clone(),
                        depth_buffer.clone(),
                        depth_buffer_gui.clone(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

pub fn indirect_buffer(
    allocator: &(impl MemoryAllocator + ?Sized),
    resource_man: &ResourceManager,
    instances: &[InstanceData],
) -> (
    Arc<CpuAccessibleBuffer<[InstanceData]>>,
    Vec<DrawIndexedIndirectCommand>,
) {
    let indirect_commands = instances
        .group_by(|a, b| a.id == b.id)
        .scan(0, |init, instances| {
            let instance_count = instances.len() as u32;

            let first_instance = *init;

            let faces = &resource_man.faces[&instances[0].id.unwrap()];
            let commands = faces
                .iter()
                .map(|face| DrawIndexedIndirectCommand {
                    index_count: face.size,
                    instance_count,
                    first_index: face.offset,
                    vertex_offset: 0,
                    first_instance,
                })
                .collect::<Vec<_>>();

            *init += instance_count;

            Some(commands)
        })
        .flatten()
        .collect::<Vec<_>>();

    let instance_buffer = cpu_accessible_buffer(
        allocator,
        instances.to_vec(),
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        },
    );

    (instance_buffer, indirect_commands)
}

pub fn indirect_instance(
    allocator: &(impl MemoryAllocator + ?Sized),
    resource_man: &ResourceManager,
    instances: &[InstanceData],
) -> Option<(
    Arc<CpuAccessibleBuffer<[DrawIndexedIndirectCommand]>>,
    Arc<CpuAccessibleBuffer<[InstanceData]>>,
)> {
    if instances.is_empty() {
        None
    } else {
        let (instance_buffer, commands) = indirect_buffer(allocator, resource_man, instances);

        if commands.is_empty() {
            None
        } else {
            let indirect_buffer = cpu_accessible_buffer(
                allocator,
                commands.into_iter(),
                BufferUsage {
                    indirect_buffer: true,
                    ..Default::default()
                },
            );
            Some((indirect_buffer, instance_buffer))
        }
    }
}

pub struct RenderAlloc {
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<SwapchainImage>>,

    pub vertex_buffer: Arc<DeviceLocalBuffer<[Vertex]>>,
    pub index_buffer: Arc<DeviceLocalBuffer<[u32]>>,
    pub game_uniform_buffer: Arc<CpuAccessibleBuffer<GameUBO>>,
    pub gui_uniform_buffer: Arc<CpuAccessibleBuffer<GuiUBO>>,

    pub color_image: Arc<AttachmentImage>,
    pub game_depth_buffer: Arc<AttachmentImage>,
    pub gui_depth_buffer: Arc<AttachmentImage>,

    pub command_allocator: StandardCommandBufferAllocator,
    pub descriptor_allocator: StandardDescriptorSetAllocator,
}

impl RenderAlloc {
    fn init_swapchain(
        window: Arc<Window>,
        surface: Arc<Surface>,
        physical_device: Arc<PhysicalDevice>,
        device: Arc<Device>,
    ) -> (Arc<Swapchain>, Vec<Arc<SwapchainImage>>) {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()
                .iter()
                .find(|(format, _color_space)| format.type_color() == Some(NumericType::UNORM))
                .unwrap()
                .0,
        );

        log::debug!("image_format: {:?}", image_format);

        Swapchain::new(
            device,
            surface,
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,

                image_format,
                image_extent: window_size_u32(window.as_ref()),

                image_usage: ImageUsage {
                    color_attachment: true,
                    transfer_dst: true,
                    ..Default::default()
                },
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),

                ..Default::default()
            },
        )
        .unwrap()
    }

    pub fn new(
        resource_man: Arc<ResourceManager>,
        device: Arc<Device>,
        queue: Arc<Queue>,
        surface: Arc<Surface>,
        window: Arc<Window>,
        physical_device: Arc<PhysicalDevice>,
    ) -> Self {
        let (swapchain, images) =
            Self::init_swapchain(window.clone(), surface, physical_device, device.clone());

        let allocator = StandardMemoryAllocator::new_default(device.clone());
        let descriptor_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_allocator = StandardCommandBufferAllocator::new(
            device,
            StandardCommandBufferAllocatorCreateInfo {
                ..Default::default()
            },
        );

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            &command_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let vertex_buffer = immutable_buffer(
            &allocator,
            resource_man.all_vertices.clone(),
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            },
            &mut command_buffer_builder,
        );

        let index_buffer = immutable_buffer(
            &allocator,
            resource_man
                .raw_faces
                .iter()
                .flatten()
                .flat_map(|v| v.indices.clone())
                .collect::<Vec<_>>(),
            BufferUsage {
                index_buffer: true,
                ..Default::default()
            },
            &mut command_buffer_builder,
        );

        let game_uniform_buffer = uniform_buffer(&allocator);
        let gui_uniform_buffer = uniform_buffer(&allocator);

        let color_image = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(window.as_ref()),
            Sample4,
            swapchain.image_format(),
            ImageUsage {
                color_attachment: true,
                ..Default::default()
            },
        )
        .unwrap();

        let game_depth_buffer = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(window.as_ref()),
            Sample4,
            Format::D32_SFLOAT,
            ImageUsage {
                depth_stencil_attachment: true,
                ..Default::default()
            },
        )
        .unwrap();

        let gui_depth_buffer = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(&window),
            Sample4,
            Format::D32_SFLOAT,
            ImageUsage {
                depth_stencil_attachment: true,
                ..Default::default()
            },
        )
        .unwrap();

        let command_buffer = command_buffer_builder.build().unwrap();

        block_on(
            command_buffer
                .execute(queue)
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap(),
        )
        .unwrap();

        drop(allocator);

        Self {
            swapchain,
            images,

            vertex_buffer,
            index_buffer,
            game_uniform_buffer,
            gui_uniform_buffer,

            color_image,
            game_depth_buffer,
            gui_depth_buffer,

            command_allocator,
            descriptor_allocator,
        }
    }
}

pub struct Gpu {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface>,
    pub render_pass: Arc<RenderPass>,
    pub window: Arc<Window>,

    pub game_pipeline: Arc<GraphicsPipeline>,
    pub gui_pipeline: Arc<GraphicsPipeline>,

    pub game_subpass: Subpass,
    pub gui_subpass: Subpass,

    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub alloc: RenderAlloc,
}

impl Gpu {
    pub fn resize_images(
        &mut self,
        allocator: &impl MemoryAllocator,
        dimensions: [u32; 2],
        recreate_swapchain: &mut bool,
    ) {
        Self::resize_image_with_samples(
            dimensions,
            allocator,
            Sample4,
            &mut self.alloc.color_image,
            recreate_swapchain,
        );
        Self::resize_image_with_samples(
            dimensions,
            allocator,
            Sample4,
            &mut self.alloc.game_depth_buffer,
            recreate_swapchain,
        );
        Self::resize_image_with_samples(
            dimensions,
            allocator,
            Sample4,
            &mut self.alloc.gui_depth_buffer,
            recreate_swapchain,
        );
    }

    fn resize_image_with_samples(
        size: [u32; 2],
        allocator: &impl MemoryAllocator,
        sample_count: SampleCount,
        image: &mut Arc<AttachmentImage>,
        recreate_swapchain: &mut bool,
    ) {
        if size != image.dimensions().width_height() {
            *recreate_swapchain = true;

            *image = AttachmentImage::multisampled_with_usage(
                allocator,
                size,
                sample_count,
                image.format(),
                *image.usage(),
            )
            .unwrap();
        }
    }

    fn resize_image(
        size: [u32; 2],
        allocator: &impl MemoryAllocator,
        image: &mut Arc<AttachmentImage>,
        recreate_swapchain: &mut bool,
    ) {
        if size != image.dimensions().width_height() {
            *recreate_swapchain = true;

            *image = AttachmentImage::with_usage(allocator, size, image.format(), *image.usage())
                .unwrap();
        }
    }

    pub fn recreate_swapchain(&mut self, size: [u32; 2], recreate_swapchain: &mut bool) {
        let (new_swapchain, new_images) = {
            match self.alloc.swapchain.recreate(SwapchainCreateInfo {
                image_extent: size,
                ..self.alloc.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("failed to recreate swapchain: {e:?}"),
            }
        };

        self.alloc.swapchain = new_swapchain;
        self.framebuffers = framebuffers(
            &new_images,
            self.render_pass.clone(),
            self.alloc.color_image.clone(),
            self.alloc.game_depth_buffer.clone(),
            self.alloc.gui_depth_buffer.clone(),
        );
        *recreate_swapchain = false;
    }

    pub fn commit_commands(
        &self,
        image_num: usize,
        acquire_future: SwapchainAcquireFuture,
        command_buffer: PrimaryAutoCommandBuffer,
        previous_frame_end: &mut Option<Box<dyn GpuFuture + Send + Sync>>,
        recreate_swapchain: &mut bool,
    ) {
        let future = previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.alloc.swapchain.clone(),
                    image_num as u32,
                ),
            )
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

    fn create_render_pass(swapchain: Arc<Swapchain>, device: Arc<Device>) -> Arc<RenderPass> {
        vulkano::ordered_passes_renderpass!(
            device,
            attachments: {
                color_resolve: {
                    load: DontCare,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 4,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D32_SFLOAT,
                    samples: 4,
                },
                depth_gui: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D32_SFLOAT,
                    samples: 4,
                }
            },
            passes: [
                { color: [color], depth_stencil: { depth     }, input: [], resolve: [color_resolve] },
                { color: [color], depth_stencil: { depth_gui }, input: [], resolve: [color_resolve] }
            ]
        ).unwrap()
    }

    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        surface: Arc<Surface>,
        window: Arc<Window>,
        alloc: RenderAlloc,
    ) -> Self {
        let render_pass = Self::create_render_pass(alloc.swapchain.clone(), device.clone());
        let game_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let gui_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let game_pipeline = create_game_pipeline(device.clone(), &game_subpass);
        let gui_pipeline = create_gui_pipeline(device.clone(), &gui_subpass);

        let framebuffers = framebuffers(
            &alloc.images,
            render_pass.clone(),
            alloc.color_image.clone(),
            alloc.game_depth_buffer.clone(),
            alloc.gui_depth_buffer.clone(),
        );

        Self {
            device,
            queue,
            surface,
            render_pass,
            window,

            game_pipeline,
            gui_pipeline,

            game_subpass,
            gui_subpass,

            framebuffers,
            alloc,
        }
    }
}
