use std::rc::Rc;
use std::sync::Arc;

use slice_group_by::GroupBy;
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirect};
use wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferUsages, Color, ColorTargetState, ColorWrites,
    CompareFunction, DepthStencilState, Device, DeviceDescriptor, Extent3d, Features, FilterMode,
    FragmentState, FrontFace, Instance, InstanceDescriptor, Limits, MultisampleState,
    PipelineLayoutDescriptor, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    Queue, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexState,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use automancy_defs::bytemuck;
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameUBO, OverlayUBO, PostEffectsUBO, RawInstanceData, Vertex};
use automancy_macros::OptionGetter;
use automancy_resources::ResourceManager;

pub const GPU_BACKENDS: Backends = Backends::all();

pub fn device_descriptor() -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        features: Features::INDIRECT_FIRST_INSTANCE | Features::MULTI_DRAW_INDIRECT,
        // WebGL doesn't support all of wgpu's features, so if
        // we're building for the web we'll have to disable some.
        limits: if cfg!(target_arch = "wasm32") {
            Limits::downlevel_webgl2_defaults()
        } else {
            Limits::default()
        },
        label: None,
    }
}

pub const NORMAL_CLEAR: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub const UPSCALE_LEVEL: u32 = 2;

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const SCREENSHOT_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub fn indirect_instance(
    device: &Device,
    queue: &Queue,
    resource_man: &ResourceManager,
    raw_instances: &[(RawInstanceData, Id)],
    instance_buffer: &mut Buffer,
    indirect_buffer: &mut Buffer,
) -> u32 {
    if raw_instances.is_empty() {
        return 0;
    }

    let mut instances = vec![];
    let mut ids = vec![];

    raw_instances
        .exponential_group_by(|a, b| a.1 == b.1)
        .for_each(|v| {
            instances.append(&mut v.iter().map(|v| v.0).collect::<Vec<_>>());
            ids.push(v.iter().map(|v| v.1).collect::<Vec<_>>())
        });

    let mut indirect_commands = vec![];

    let count = ids.len();

    if count == 0 {
        return 0;
    }

    ids.into_iter()
        .scan(0, |init, ids| {
            let instance_count = ids.len() as u32;

            let index_range = resource_man.index_ranges[&ids[0]];

            let command = DrawIndexedIndirect {
                base_index: index_range.offset,
                vertex_offset: 0,
                vertex_count: index_range.size,
                base_instance: *init,
                instance_count,
            };

            *init += instance_count;

            Some(command)
        })
        .for_each(|command| indirect_commands.extend_from_slice(command.as_bytes()));

    create_or_write_buffer(
        device,
        queue,
        instance_buffer,
        bytemuck::cast_slice(instances.as_slice()),
    );
    create_or_write_buffer(device, queue, indirect_buffer, &indirect_commands);

    count as u32
}

pub fn create_or_write_buffer(
    device: &Device,
    queue: &Queue,
    buffer: &mut Buffer,
    contents: &[u8],
) {
    if buffer.size() < contents.len() as BufferAddress {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents,
            usage,
        })
    } else {
        queue.write_buffer(buffer, 0, contents);
    }
}

pub fn create_surface_texture(
    device: &Device,
    format: TextureFormat,
    size: Extent3d,
    label: Option<&str>,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

pub fn create_texture(
    device: &Device,
    format: TextureFormat,
    dimension: TextureDimension,
    size: Extent3d,
    label: Option<&str>,
    usage: TextureUsages,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension,
        format,
        usage,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

pub fn create_texture_init(
    device: &Device,
    queue: &Queue,
    format: TextureFormat,
    dimension: TextureDimension,
    size: Extent3d,
    usage: TextureUsages,
    data: &[u8],
) -> (Texture, TextureView) {
    let texture = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            usage,
            view_formats: &[],
        },
        data,
    );

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

fn extent3d(config: &SurfaceConfiguration, scale: u32) -> Extent3d {
    Extent3d {
        width: config.width * scale,
        height: config.height * scale,
        depth_or_array_layers: 1,
    }
}

fn game_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
) -> (Buffer, BindGroup, RenderPipeline) {
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Game Uniform Buffer"),
        contents: bytemuck::cast_slice(&[GameUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("game_bind_group_layout"),
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("game_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Game Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Game Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc(), RawInstanceData::desc()],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[
                Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            ],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    (uniform_buffer, bind_group, pipeline)
}

fn gui_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
) -> (Buffer, BindGroup, RenderPipeline) {
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Gui Uniform Buffer"),
        contents: bytemuck::cast_slice(&[GameUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("gui_bind_group_layout"),
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("gui_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Gui Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Gui Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc(), RawInstanceData::desc()],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[
                Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            ],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    (uniform_buffer, bind_group, pipeline)
}

fn extra_setup(device: &Device) -> Buffer {
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Extra Uniform Buffer"),
        contents: bytemuck::cast_slice(&[GameUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    uniform_buffer
}

fn make_post_effects_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 6,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
        label: Some("post_effects_bind_group_layout"),
    })
}

fn make_post_effects_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    uniform_buffer: &Buffer,
    surface_texture: &TextureView,
    surface_sampler: &Sampler,
    normal_texture: &TextureView,
    normal_sampler: &Sampler,
    depth_texture: &TextureView,
    depth_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(surface_texture),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Sampler(surface_sampler),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(normal_texture),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::Sampler(normal_sampler),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::TextureView(depth_texture),
            },
            BindGroupEntry {
                binding: 6,
                resource: BindingResource::Sampler(depth_sampler),
            },
        ],
        label: Some("post_effects_bind_group"),
    })
}

fn post_effects_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
    bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Post Effects Render Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Post Effects Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    pipeline
}

fn overlay_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
) -> (Buffer, BindGroup, RenderPipeline) {
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Overlay Uniform Buffer"),
        contents: bytemuck::cast_slice(&[OverlayUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("overlay_bind_group_layout"),
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("overlay_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Overlay Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Overlay Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    (uniform_buffer, bind_group, pipeline)
}

fn combine_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("combine_bind_group_layout"),
    })
}

fn make_combine_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    a_texture: &TextureView,
    a_sampler: &Sampler,
    b_texture: &TextureView,
    b_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(a_texture),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(a_sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(b_texture),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(b_sampler),
            },
        ],
        label: Some("combine_bind_group"),
    })
}

fn combine_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
    bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Combine Render Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Combine Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    pipeline
}

#[derive(OptionGetter)]
pub struct GameResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
    pub post_effects_uniform_buffer: Buffer,
    #[getters(get)]
    post_effects_bind_group: Option<BindGroup>,
}

#[derive(OptionGetter)]
pub struct GuiResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
    pub post_effects_uniform_buffer: Buffer,
    #[getters(get)]
    post_effects_bind_group: Option<BindGroup>, //TODO combine with game
}

#[derive(OptionGetter)]
pub struct EguiResources {
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
}

#[derive(OptionGetter)]
pub struct ExtraResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
}

pub struct OverlayResources {
    pub vertex_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
}

#[derive(OptionGetter)]
pub struct CombineResources {
    pub bind_group_layout: Rc<BindGroupLayout>,
    pub pipeline: Rc<RenderPipeline>,
    #[getters(get)]
    bind_group: Option<BindGroup>,
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
}

#[derive(OptionGetter)]
pub struct PostEffectsResources {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
}

#[derive(OptionGetter)]
pub struct IntermediateResources {
    pub bind_group_layout: BindGroupLayout,
    pub screenshot_pipeline: RenderPipeline,
    pub scale_pipeline: RenderPipeline,
}

pub struct Gpu {
    vsync: bool,
    new_size: Option<PhysicalSize<u32>>,

    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub config: SurfaceConfiguration,
    pub window: Arc<Window>,

    pub game_shader: ShaderModule,
    pub overlay_shader: ShaderModule,
    pub post_effects_shader: ShaderModule,
    pub combine_shader: ShaderModule,
    pub intermediate_shader: ShaderModule,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    game_texture: Option<(Texture, TextureView)>,
    normal_texture: Option<(Texture, TextureView)>,
    depth_texture: Option<(Texture, TextureView)>,

    pub filtering_sampler: Sampler,
    pub non_filtering_sampler: Sampler,

    pub game_resources: GameResources,
    pub gui_resources: GuiResources,
    pub egui_resources: EguiResources,
    pub extra_resources: ExtraResources,
    pub overlay_resources: OverlayResources,
    pub first_combine_resources: CombineResources,
    pub post_effects_resources: PostEffectsResources,
    pub second_combine_resources: CombineResources,
    pub intermediate_resources: IntermediateResources,
}

impl Gpu {
    pub fn game_texture(&self) -> &(Texture, TextureView) {
        self.game_texture.as_ref().unwrap()
    }

    pub fn normal_texture(&self) -> &(Texture, TextureView) {
        self.normal_texture.as_ref().unwrap()
    }

    pub fn depth_texture(&self) -> &(Texture, TextureView) {
        self.depth_texture.as_ref().unwrap()
    }

    fn pick_present_mode(vsync: bool) -> PresentMode {
        if vsync {
            PresentMode::Fifo
        } else {
            PresentMode::AutoNoVsync
        }
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        if self.vsync != vsync {
            self.vsync = vsync;
            self.config.present_mode = Self::pick_present_mode(vsync);

            self.surface.configure(&self.device, &self.config);
        }
    }

    pub async fn new(
        window: Window,
        resource_man: &ResourceManager,
        vertices: Vec<Vertex>,
        indices: Vec<u16>,
        vsync: bool,
    ) -> Gpu {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor {
            backends: GPU_BACKENDS,
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&device_descriptor(), None)
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let game_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["game"].as_str().into()),
        });

        let overlay_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Overlay Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["overlay"].as_str().into()),
        });

        let post_effects_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Post Effects Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["post_effects"].as_str().into()),
        });

        let combine_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Combine Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["combine"].as_str().into()),
        });

        let intermediate_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Intermediate Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["intermediate"].as_str().into()),
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: BufferUsages::INDEX,
        });

        let filtering_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let non_filtering_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let game_resources = {
            let (uniform_buffer, bind_group, pipeline) = game_setup(&device, &config, &game_shader);

            GameResources {
                instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                indirect_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
                bind_group,
                pipeline,
                post_effects_uniform_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Game Post Effects Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[PostEffectsUBO::default()]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }),
                post_effects_bind_group: None,
            }
        };

        let gui_resources = {
            let (uniform_buffer, bind_group, pipeline) = gui_setup(&device, &config, &game_shader);

            GuiResources {
                instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
                bind_group,
                pipeline,
                post_effects_uniform_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Gui Post Effects Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[PostEffectsUBO::default()]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }),
                post_effects_bind_group: None,
            }
        };

        let egui_resources = EguiResources { texture: None };

        let extra_resources = {
            let uniform_buffer = extra_setup(&device);

            ExtraResources {
                instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                indirect_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
            }
        };

        let overlay_resources = {
            let (uniform_buffer, bind_group, pipeline) =
                overlay_setup(&device, &config, &overlay_shader);

            OverlayResources {
                vertex_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
                bind_group,
                pipeline,
            }
        };

        let combine_bind_group_layout = Rc::new(combine_bind_group_layout(&device));

        let combine_pipeline = Rc::new(combine_setup(
            &device,
            &config,
            &combine_shader,
            &combine_bind_group_layout,
        ));

        let first_combine_resources = CombineResources {
            bind_group_layout: combine_bind_group_layout.clone(),
            pipeline: combine_pipeline.clone(),
            bind_group: None,
            texture: None,
        };

        let post_effects_resources = {
            let bind_group_layout = make_post_effects_bind_group_layout(&device);

            let pipeline =
                post_effects_setup(&device, &config, &post_effects_shader, &bind_group_layout);

            PostEffectsResources {
                bind_group_layout,
                pipeline,
                texture: None,
            }
        };

        let second_combine_resources = CombineResources {
            bind_group_layout: combine_bind_group_layout.clone(),
            pipeline: combine_pipeline.clone(),
            bind_group: None,
            texture: None,
        };

        let intermediate_resources = {
            let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let intermediate_pipeline_layout =
                device.create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some("Intermediate Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

            let screenshot_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Screenshot Render Pipeline"),
                layout: Some(&intermediate_pipeline_layout),
                vertex: VertexState {
                    module: &intermediate_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &intermediate_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: SCREENSHOT_FORMAT,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            let scale_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Scale Render Pipeline"),
                layout: Some(&intermediate_pipeline_layout),
                vertex: VertexState {
                    module: &intermediate_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &intermediate_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            IntermediateResources {
                bind_group_layout,
                screenshot_pipeline,
                scale_pipeline,
            }
        };

        let mut this = Self {
            vsync: false,
            new_size: None,

            instance,
            device,
            queue,
            surface,
            config,
            window: Arc::new(window),

            game_shader,
            overlay_shader,
            post_effects_shader,
            combine_shader,
            intermediate_shader,

            vertex_buffer,
            index_buffer,

            game_texture: None,
            normal_texture: None,
            depth_texture: None,

            filtering_sampler,
            non_filtering_sampler,

            game_resources,
            gui_resources,
            egui_resources,
            extra_resources,
            overlay_resources,
            first_combine_resources,
            post_effects_resources,
            second_combine_resources,
            intermediate_resources,
        };

        this.create_textures(size);

        this
    }

    pub fn take_new_size(&mut self) -> Option<PhysicalSize<u32>> {
        self.new_size.take()
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.new_size = Some(size);
    }

    pub fn create_textures(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;

        let device = &self.device;
        let config = &self.config;

        self.surface.configure(device, config);

        let original = extent3d(config, 1);
        let upscale = extent3d(config, UPSCALE_LEVEL);

        self.normal_texture = Some(create_surface_texture(
            device,
            TextureFormat::Rgba32Float,
            upscale,
            None,
        ));
        self.depth_texture = Some(create_texture(
            device,
            DEPTH_FORMAT,
            TextureDimension::D2,
            upscale,
            None,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        ));

        self.game_texture = Some(create_surface_texture(device, config.format, upscale, None));

        self.egui_resources.texture = Some(create_surface_texture(
            device,
            config.format,
            original,
            None,
        ));

        self.game_resources.post_effects_bind_group = Some(make_post_effects_bind_group(
            device,
            &self.post_effects_resources.bind_group_layout,
            &self.game_resources.post_effects_uniform_buffer,
            &self.game_texture().1,
            &self.non_filtering_sampler,
            &self.normal_texture().1,
            &self.non_filtering_sampler,
            &self.depth_texture().1,
            &self.non_filtering_sampler,
        ));

        self.gui_resources.post_effects_bind_group = Some(make_post_effects_bind_group(
            device,
            &self.post_effects_resources.bind_group_layout,
            &self.gui_resources.post_effects_uniform_buffer,
            &self.game_texture().1,
            &self.non_filtering_sampler,
            &self.normal_texture().1,
            &self.non_filtering_sampler,
            &self.depth_texture().1,
            &self.non_filtering_sampler,
        ));

        self.post_effects_resources.texture =
            Some(create_surface_texture(device, config.format, upscale, None));

        self.first_combine_resources.texture = Some(create_surface_texture(
            device,
            config.format,
            original,
            None,
        ));
        self.first_combine_resources.bind_group = Some(make_combine_bind_group(
            device,
            &self.first_combine_resources.bind_group_layout,
            &self.post_effects_resources.texture().1,
            &self.filtering_sampler,
            &self.egui_resources.texture().1,
            &self.filtering_sampler,
        ));

        self.second_combine_resources.texture = Some(create_surface_texture(
            device,
            config.format,
            original,
            None,
        ));
        self.second_combine_resources.bind_group = Some(make_combine_bind_group(
            device,
            &self.second_combine_resources.bind_group_layout,
            &self.first_combine_resources.texture().1,
            &self.filtering_sampler,
            &self.post_effects_resources.texture().1,
            &self.filtering_sampler,
        ));
    }
}
