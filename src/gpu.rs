use std::sync::Arc;

use slice_group_by::GroupBy;

use automancy_defs::cg::{Double, Float};
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameUBO, GameVertex, LightInfo, OverlayUBO, RawInstanceData};
use automancy_defs::vulkano::buffer::{Buffer, BufferCreateInfo, Subbuffer};
use automancy_defs::vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use automancy_defs::vulkano::command_buffer::PrimaryAutoCommandBuffer;
use automancy_defs::vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use automancy_defs::vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use automancy_defs::vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use automancy_defs::vulkano::device::{
    DeviceCreateInfo, DeviceExtensions, Features, QueueCreateInfo, QueueFlags,
};
use automancy_defs::vulkano::format::{Format, NumericType};
use automancy_defs::vulkano::image::{ImageAccess, ImageUsage, SampleCount, SampleCount::Sample4};
use automancy_defs::vulkano::instance::{Instance, InstanceCreateInfo};
use automancy_defs::vulkano::memory::allocator::{
    AllocationCreateInfo, MemoryAllocator, MemoryUsage, StandardMemoryAllocator,
};
use automancy_defs::vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use automancy_defs::vulkano::pipeline::graphics::input_assembly::{
    InputAssemblyState, PrimitiveTopology,
};
use automancy_defs::vulkano::pipeline::graphics::multisample::MultisampleState;
use automancy_defs::vulkano::pipeline::graphics::rasterization::RasterizationState;
use automancy_defs::vulkano::pipeline::graphics::vertex_input::Vertex;
use automancy_defs::vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use automancy_defs::vulkano::pipeline::{GraphicsPipeline, Pipeline};
use automancy_defs::vulkano::render_pass::Subpass;
use automancy_defs::vulkano::swapchain::{
    Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError,
    SwapchainPresentInfo,
};
use automancy_defs::vulkano::sync::FlushError;
use automancy_defs::vulkano::{
    buffer::BufferUsage,
    command_buffer::DrawIndexedIndirectCommand,
    device::{Device, Queue},
    image::{view::ImageView, AttachmentImage, SwapchainImage},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::Surface,
    sync,
    sync::GpuFuture,
    VulkanLibrary,
};
use automancy_defs::vulkano_win::create_surface_from_winit;
use automancy_defs::winit::event_loop::EventLoop;
use automancy_defs::winit::window::{Icon, WindowBuilder};
use automancy_defs::winit::{dpi::LogicalSize, window::Window};
use automancy_defs::{log, shaders, vulkano_win};
use automancy_resources::ResourceManager;

fn create_render_pass(swapchain: Arc<Swapchain>, device: Arc<Device>) -> Arc<RenderPass> {
    automancy_defs::vulkano::ordered_passes_renderpass!(
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
            depth_game: {
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
            { color: [color], depth_stencil: { depth_game }, input: [], resolve: [color_resolve] },
            { color: [color], depth_stencil: { depth_gui }, input: [], resolve: [color_resolve] }
        ]
    )
    .unwrap()
}

pub fn create_game_pipeline(device: Arc<Device>, subpass: &Subpass) -> Arc<GraphicsPipeline> {
    let vs = shaders::game_vert_shader::load(device.clone()).unwrap();
    let fs = shaders::game_frag_shader::load(device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .vertex_input_state([GameVertex::per_vertex(), RawInstanceData::per_instance()])
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
    let vs = shaders::gui_vert_shader::load(device.clone()).unwrap();
    let fs = shaders::gui_frag_shader::load(device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .vertex_input_state([
            GameVertex::per_vertex(),
            RawInstanceData::per_instance(),
            LightInfo::per_instance(),
        ])
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
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

pub fn create_overlay_pipeline(device: Arc<Device>, subpass: &Subpass) -> Arc<GraphicsPipeline> {
    let vs = shaders::overlay_vert_shader::load(device.clone()).unwrap();
    let fs = shaders::overlay_frag_shader::load(device.clone()).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .vertex_input_state(GameVertex::per_vertex())
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
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
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

    viewport_with_dims([width as Float, height as Float])
}

pub fn viewport_with_dims(dimensions: [Float; 2]) -> Viewport {
    Viewport {
        origin: [0.0, 0.0],
        dimensions,
        depth_range: 0.0..1.0,
    }
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

type IndirectInstanceBuffers = (
    Subbuffer<[DrawIndexedIndirectCommand]>,
    Subbuffer<[RawInstanceData]>,
);

pub fn indirect_instance(
    allocator: &(impl MemoryAllocator + ?Sized),
    resource_man: &ResourceManager,
    instances: &[(RawInstanceData, Id)],
) -> Option<IndirectInstanceBuffers> {
    if instances.is_empty() {
        return None;
    }

    let (indirect_commands, instance_buffer) = {
        let indirect_commands = instances
            .exponential_group_by(|a, b| a.1 == b.1)
            .scan(0, |init, instances| {
                let instance_count = instances.len() as u32;

                let mesh = &resource_man.meshes[&instances[0].1];

                let command = DrawIndexedIndirectCommand {
                    index_count: mesh.size,
                    instance_count,
                    first_index: mesh.offset,
                    vertex_offset: 0,
                    first_instance: *init,
                };

                *init += instance_count;

                Some(command)
            })
            .collect::<Vec<_>>();

        let instance_buffer = Buffer::from_iter(
            allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            instances.iter().map(|v| v.0),
        )
        .unwrap();

        (indirect_commands, instance_buffer)
    };

    if indirect_commands.is_empty() {
        return None;
    }

    let indirect_buffer = Buffer::from_iter(
        allocator,
        BufferCreateInfo {
            usage: BufferUsage::INDIRECT_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        indirect_commands.into_iter(),
    )
    .unwrap();

    Some((indirect_buffer, instance_buffer))
}

pub struct RenderAlloc {
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<SwapchainImage>>,

    pub vertex_buffer: Subbuffer<[GameVertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub game_uniform_buffer: Subbuffer<GameUBO>,
    pub overlay_uniform_buffer: Subbuffer<OverlayUBO>,

    pub color_image: Arc<AttachmentImage>,
    pub game_depth_buffer: Arc<AttachmentImage>,
    pub gui_depth_buffer: Arc<AttachmentImage>,

    pub allocator: StandardMemoryAllocator,
    pub command_allocator: StandardCommandBufferAllocator,

    pub physical_device: Arc<PhysicalDevice>,
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
                .find(|(format, _color_space)| format.type_color() == Some(NumericType::SRGB))
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

                image_usage: ImageUsage::COLOR_ATTACHMENT.union(ImageUsage::TRANSFER_DST),
                ..Default::default()
            },
        )
        .unwrap()
    }

    pub fn new(
        resource_man: &ResourceManager,
        device: Arc<Device>,
        surface: Arc<Surface>,
        window: Arc<Window>,
        physical_device: Arc<PhysicalDevice>,
    ) -> Self {
        let (swapchain, images) = Self::init_swapchain(
            window.clone(),
            surface,
            physical_device.clone(),
            device.clone(),
        );

        let allocator = StandardMemoryAllocator::new_default(device.clone());
        let command_allocator = StandardCommandBufferAllocator::new(
            device,
            StandardCommandBufferAllocatorCreateInfo {
                ..Default::default()
            },
        );

        let vertex_buffer = Buffer::from_iter(
            &allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            resource_man.all_vertices.iter().cloned(),
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            &allocator,
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            resource_man
                .faces
                .iter()
                .flat_map(|v| v.indices.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();

        let game_uniform_buffer = Buffer::from_data(
            &allocator,
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            GameUBO::default(),
        )
        .unwrap();

        let overlay_uniform_buffer = Buffer::from_data(
            &allocator,
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            OverlayUBO::default(),
        )
        .unwrap();

        let color_image = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(window.as_ref()),
            Sample4,
            swapchain.image_format(),
            ImageUsage::COLOR_ATTACHMENT,
        )
        .unwrap();

        let game_depth_buffer = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(window.as_ref()),
            Sample4,
            Format::D32_SFLOAT,
            ImageUsage::DEPTH_STENCIL_ATTACHMENT,
        )
        .unwrap();

        let gui_depth_buffer = AttachmentImage::multisampled_with_usage(
            &allocator,
            window_size_u32(&window),
            Sample4,
            Format::D32_SFLOAT,
            ImageUsage::DEPTH_STENCIL_ATTACHMENT,
        )
        .unwrap();

        Self {
            swapchain,
            images,

            vertex_buffer,
            index_buffer,
            game_uniform_buffer,
            overlay_uniform_buffer,

            color_image,
            game_depth_buffer,
            gui_depth_buffer,

            allocator,
            command_allocator,

            physical_device,
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
    pub overlay_pipeline: Arc<GraphicsPipeline>,

    pub game_descriptor_set: Arc<PersistentDescriptorSet>,
    pub overlay_descriptor_set: Arc<PersistentDescriptorSet>,

    pub game_subpass: Subpass,
    pub gui_subpass: Subpass,

    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub alloc: RenderAlloc,
}

impl Gpu {
    pub fn resize_images(&mut self, dimensions: [u32; 2], recreate_swapchain: &mut bool) {
        Self::resize_image_with_samples(
            dimensions,
            &self.alloc.allocator,
            Sample4,
            &mut self.alloc.color_image,
            recreate_swapchain,
        );
        Self::resize_image_with_samples(
            dimensions,
            &self.alloc.allocator,
            Sample4,
            &mut self.alloc.game_depth_buffer,
            recreate_swapchain,
        );
        Self::resize_image_with_samples(
            dimensions,
            &self.alloc.allocator,
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
                image.usage(),
            )
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

    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        surface: Arc<Surface>,
        window: Arc<Window>,
        alloc: RenderAlloc,
    ) -> Self {
        let render_pass = create_render_pass(alloc.swapchain.clone(), device.clone());
        let game_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let gui_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let game_pipeline = create_game_pipeline(device.clone(), &game_subpass);
        let gui_pipeline = create_gui_pipeline(device.clone(), &gui_subpass);
        let overlay_pipeline = create_overlay_pipeline(device.clone(), &gui_subpass);

        let descriptor_allocator = StandardDescriptorSetAllocator::new(device.clone());

        let game_descriptor_set = PersistentDescriptorSet::new(
            &descriptor_allocator,
            game_pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(
                0,
                alloc.game_uniform_buffer.clone(),
            )],
        )
        .unwrap();

        let overlay_descriptor_set = PersistentDescriptorSet::new(
            &descriptor_allocator,
            overlay_pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(
                0,
                alloc.overlay_uniform_buffer.clone(),
            )],
        )
        .unwrap();

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
            overlay_pipeline,

            game_descriptor_set,
            overlay_descriptor_set,

            game_subpass,
            gui_subpass,

            framebuffers,
            alloc,
        }
    }
}
