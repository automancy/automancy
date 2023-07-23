use std::sync::Arc;

use slice_group_by::GroupBy;
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirect};
use wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
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
use automancy_defs::rendering::{GameUBO, OverlayUBO, RawInstanceData, Vertex};
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

fn game_shader(device: &Device, resource_man: &ResourceManager) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Game Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["game"].as_str().into()),
    })
}

fn effects_shader(device: &Device, resource_man: &ResourceManager) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Effects Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["effects"].as_str().into()),
    })
}

fn overlay_shader(device: &Device, resource_man: &ResourceManager) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Overlay Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["overlay"].as_str().into()),
    })
}

fn combine_shader(device: &Device, resource_man: &ResourceManager) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Combine Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["combine"].as_str().into()),
    })
}

pub const UPSCALE_LEVEL: u32 = 2;

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub const GAME_TEXTURE: Option<&str> = Some("Game Texture");
pub const GAME_POSITION_TEXTURE: Option<&str> = Some("Game Position Texture");
pub const GAME_NORMAL_TEXTURE: Option<&str> = Some("Game Normal Texture");
pub const GAME_DEPTH_TEXTURE: Option<&str> = Some("Game Depth Texture");
pub const PROCESSED_GAME_TEXTURE: Option<&str> = Some("Processed Game Texture");
pub const GUI_TEXTURE: Option<&str> = Some("Gui Texture");
pub const GUI_POSITION_TEXTURE: Option<&str> = Some("Gui Position Texture");
pub const GUI_NORMAL_TEXTURE: Option<&str> = Some("Gui Normal Texture");
pub const GUI_DEPTH_TEXTURE: Option<&str> = Some("Gui Depth Texture");
pub const PROCESSED_GUI_TEXTURE: Option<&str> = Some("Processed Gui Texture");
pub const EGUI_TEXTURE: Option<&str> = Some("Egui Texture");

pub const GAME_INDIRECT_BUFFER: Option<&str> = Some("Game Indirect Buffer");
pub const GAME_INSTANCE_BUFFER: Option<&str> = Some("Game Instance Buffer");

pub const EXTRA_INDIRECT_BUFFER: Option<&str> = Some("Extra Indirect Buffer");
pub const EXTRA_INSTANCE_BUFFER: Option<&str> = Some("Extra Instance Buffer");

pub const GUI_INSTANCE_BUFFER: Option<&str> = Some("Gui Instance Buffer");

pub const OVERLAY_VERTEX_BUFFER: Option<&str> = Some("Overlay Vertex Buffer");

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
        GAME_INSTANCE_BUFFER,
        bytemuck::cast_slice(instances.as_slice()),
    );
    create_or_write_buffer(
        device,
        queue,
        indirect_buffer,
        GAME_INDIRECT_BUFFER,
        &indirect_commands,
    );

    count as u32
}

pub fn create_or_write_buffer(
    device: &Device,
    queue: &Queue,
    buffer: &mut Buffer,
    label: Option<&'static str>,
    contents: &[u8],
) {
    if buffer.size() < contents.len() as BufferAddress {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
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
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

pub fn create_texture_init(
    device: &Device,
    queue: &Queue,
    format: TextureFormat,
    size: Extent3d,
    label: Option<&str>,
    data: &[u8],
) -> (Texture, TextureView) {
    let texture = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
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
                Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
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
                Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
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

fn make_effects_bind_group_layout(device: &Device) -> BindGroupLayout {
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
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
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
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
        label: Some("effects_bind_group_layout"),
    })
}

fn make_effects_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    uniform_buffer: &Buffer,
    surface_texture: &TextureView,
    position_texture: &TextureView,
    normal_texture: &TextureView,
    noise_texture: &TextureView,
    noise_texture_sampler: &Sampler,
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
                resource: BindingResource::TextureView(position_texture),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(normal_texture),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(noise_texture),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::Sampler(noise_texture_sampler),
            },
        ],
        label: Some("effects_bind_group"),
    })
}

fn effects_setup(
    device: &Device,
    config: &SurfaceConfiguration,
    shader: &ShaderModule,
    bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Effects Render Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Effects Render Pipeline"),
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
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    (uniform_buffer, bind_group, pipeline)
}

fn make_combine_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    processed_game_texture: &TextureView,
    processed_game_sampler: &Sampler,
    gui_texture: &TextureView,
    gui_sampler: &Sampler,
    egui_texture: &TextureView,
    egui_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(processed_game_texture),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(processed_game_sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(gui_texture),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(gui_sampler),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(egui_texture),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::Sampler(egui_sampler),
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

pub struct Gpu {
    vsync: bool,

    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub config: SurfaceConfiguration,
    pub window: Arc<Window>,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    pub game_texture: (Texture, TextureView),
    pub game_position_texture: (Texture, TextureView),
    pub game_normal_texture: (Texture, TextureView),
    pub game_depth_texture: (Texture, TextureView),
    pub processed_game_texture: (Texture, TextureView),
    pub processed_game_sampler: Sampler,

    pub gui_texture: (Texture, TextureView),
    pub gui_position_texture: (Texture, TextureView),
    pub gui_normal_texture: (Texture, TextureView),
    pub gui_depth_texture: (Texture, TextureView),
    pub processed_gui_texture: (Texture, TextureView),
    pub processed_gui_sampler: Sampler,

    pub egui_texture: (Texture, TextureView),
    pub egui_sampler: Sampler,

    pub game_instance_buffer: Buffer,
    pub game_indirect_buffer: Buffer,
    pub game_uniform_buffer: Buffer,
    pub game_bind_group: BindGroup,
    pub game_pipeline: RenderPipeline,

    pub extra_instance_buffer: Buffer,
    pub extra_indirect_buffer: Buffer,
    pub extra_uniform_buffer: Buffer,

    pub effects_bind_group_layout: BindGroupLayout,
    pub effects_noise_texture: (Texture, TextureView),
    pub effects_noise_sampler: Sampler,
    pub game_effects_bind_group: BindGroup,
    pub gui_effects_bind_group: BindGroup,
    pub effects_pipeline: RenderPipeline,

    pub gui_instance_buffer: Buffer,
    pub gui_uniform_buffer: Buffer,
    pub gui_bind_group: BindGroup,
    pub gui_pipeline: RenderPipeline,

    pub overlay_vertex_buffer: Buffer,
    pub overlay_uniform_buffer: Buffer,
    pub overlay_bind_group: BindGroup,
    pub overlay_pipeline: RenderPipeline,

    pub combine_bind_group_layout: BindGroupLayout,
    pub combine_bind_group: BindGroup,
    pub combine_pipeline: RenderPipeline,
}

impl Gpu {
    fn pick_present_mode(vsync: bool) -> PresentMode {
        if vsync {
            PresentMode::AutoVsync
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;

        let extent = extent3d(&self.config, UPSCALE_LEVEL);

        self.game_texture =
            create_surface_texture(&self.device, self.config.format, extent, GAME_TEXTURE);
        self.game_position_texture = create_surface_texture(
            &self.device,
            TextureFormat::Rgba32Float,
            extent,
            GAME_POSITION_TEXTURE,
        );
        self.game_normal_texture = create_surface_texture(
            &self.device,
            TextureFormat::Rgba8Unorm,
            extent,
            GAME_NORMAL_TEXTURE,
        );
        self.game_depth_texture =
            create_texture(&self.device, DEPTH_FORMAT, extent, GAME_DEPTH_TEXTURE);
        self.processed_game_texture = create_surface_texture(
            &self.device,
            self.config.format,
            extent,
            PROCESSED_GAME_TEXTURE,
        );

        self.gui_texture =
            create_surface_texture(&self.device, self.config.format, extent, GUI_TEXTURE);
        self.gui_position_texture = create_surface_texture(
            &self.device,
            TextureFormat::Rgba32Float,
            extent,
            GUI_POSITION_TEXTURE,
        );
        self.gui_normal_texture = create_surface_texture(
            &self.device,
            TextureFormat::Rgba8Unorm,
            extent,
            GUI_NORMAL_TEXTURE,
        );
        self.gui_depth_texture =
            create_texture(&self.device, DEPTH_FORMAT, extent, GUI_DEPTH_TEXTURE);
        self.processed_gui_texture = create_surface_texture(
            &self.device,
            self.config.format,
            extent,
            PROCESSED_GUI_TEXTURE,
        );
        self.egui_texture = create_surface_texture(
            &self.device,
            self.config.format,
            extent3d(&self.config, 1),
            EGUI_TEXTURE,
        );

        self.game_effects_bind_group = make_effects_bind_group(
            &self.device,
            &self.effects_bind_group_layout,
            &self.game_uniform_buffer,
            &self.game_texture.1,
            &self.game_position_texture.1,
            &self.game_normal_texture.1,
            &self.effects_noise_texture.1,
            &self.effects_noise_sampler,
        );
        self.gui_effects_bind_group = make_effects_bind_group(
            &self.device,
            &self.effects_bind_group_layout,
            &self.gui_uniform_buffer,
            &self.gui_texture.1,
            &self.gui_position_texture.1,
            &self.gui_normal_texture.1,
            &self.effects_noise_texture.1,
            &self.effects_noise_sampler,
        );

        self.combine_bind_group = make_combine_bind_group(
            &self.device,
            &self.combine_bind_group_layout,
            &self.processed_game_texture.1,
            &self.processed_game_sampler,
            &self.processed_gui_texture.1,
            &self.processed_gui_sampler,
            &self.egui_texture.1,
            &self.egui_sampler,
        );

        self.surface.configure(&self.device, &self.config);
    }

    pub async fn new(
        window: Window,
        resource_man: &ResourceManager,
        vertices: Vec<Vertex>,
        indices: Vec<u16>,
        vsync: bool,
    ) -> Self {
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
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        let extent = extent3d(&config, UPSCALE_LEVEL);

        surface.configure(&device, &config);

        let sampler_desc = SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        };

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

        let game_texture = create_surface_texture(&device, config.format, extent, GAME_TEXTURE);
        let game_position_texture = create_surface_texture(
            &device,
            TextureFormat::Rgba32Float,
            extent,
            GAME_POSITION_TEXTURE,
        );
        let game_normal_texture = create_surface_texture(
            &device,
            TextureFormat::Rgba8Unorm,
            extent,
            GAME_NORMAL_TEXTURE,
        );
        let game_depth_texture = create_texture(&device, DEPTH_FORMAT, extent, GAME_DEPTH_TEXTURE);
        let processed_game_texture =
            create_surface_texture(&device, config.format, extent, PROCESSED_GAME_TEXTURE);
        let processed_game_sampler = device.create_sampler(&sampler_desc);

        let gui_texture = create_surface_texture(&device, config.format, extent, GUI_TEXTURE);
        let gui_position_texture = create_surface_texture(
            &device,
            TextureFormat::Rgba32Float,
            extent,
            GUI_POSITION_TEXTURE,
        );
        let gui_normal_texture = create_surface_texture(
            &device,
            TextureFormat::Rgba8Unorm,
            extent,
            GUI_NORMAL_TEXTURE,
        );
        let gui_depth_texture = create_texture(&device, DEPTH_FORMAT, extent, GUI_DEPTH_TEXTURE);
        let processed_gui_texture =
            create_surface_texture(&device, config.format, extent, PROCESSED_GUI_TEXTURE);
        let processed_gui_sampler = device.create_sampler(&sampler_desc);

        let egui_texture =
            create_surface_texture(&device, config.format, extent3d(&config, 1), EGUI_TEXTURE);
        let egui_sampler = device.create_sampler(&sampler_desc);

        let game_instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: GAME_INSTANCE_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let game_indirect_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: GAME_INDIRECT_BUFFER,
            contents: &[],
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
        });

        let extra_instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: EXTRA_INSTANCE_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let extra_indirect_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: EXTRA_INDIRECT_BUFFER,
            contents: &[],
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
        });

        let gui_instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: GUI_INSTANCE_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let overlay_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: OVERLAY_VERTEX_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let (game_uniform_buffer, game_bind_group, game_pipeline) =
            game_setup(&device, &config, &game_shader(&device, resource_man));
        let (gui_uniform_buffer, gui_bind_group, gui_pipeline) =
            gui_setup(&device, &config, &game_shader(&device, resource_man));
        let extra_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Extra Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let (overlay_uniform_buffer, overlay_bind_group, overlay_pipeline) =
            overlay_setup(&device, &config, &overlay_shader(&device, resource_man));

        let effects_bind_group_layout = make_effects_bind_group_layout(&device);

        let noise_texture_data: [[u8; 4]; 16] = [
            [163, 180, 0, 0],
            [32, 55, 0, 0],
            [127, 235, 0, 0],
            [40, 14, 0, 0],
            [246, 185, 0, 0],
            [140, 72, 0, 0],
            [9, 78, 0, 0],
            [114, 107, 0, 0],
            [90, 209, 0, 0],
            [166, 44, 0, 0],
            [69, 29, 0, 0],
            [97, 32, 0, 0],
            [61, 76, 0, 0],
            [19, 163, 0, 0],
            [202, 33, 0, 0],
            [185, 156, 0, 0],
        ];
        let effects_noise_texture = create_texture_init(
            &device,
            &queue,
            TextureFormat::Rgba8Unorm,
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            None,
            bytemuck::cast_slice(noise_texture_data.as_slice()),
        );
        let effects_noise_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let game_effects_bind_group = make_effects_bind_group(
            &device,
            &effects_bind_group_layout,
            &game_uniform_buffer,
            &game_texture.1,
            &game_position_texture.1,
            &game_normal_texture.1,
            &effects_noise_texture.1,
            &effects_noise_sampler,
        );
        let gui_effects_bind_group = make_effects_bind_group(
            &device,
            &effects_bind_group_layout,
            &gui_uniform_buffer,
            &gui_texture.1,
            &gui_position_texture.1,
            &gui_normal_texture.1,
            &effects_noise_texture.1,
            &effects_noise_sampler,
        );

        let effects_pipeline = effects_setup(
            &device,
            &config,
            &effects_shader(&device, resource_man),
            &effects_bind_group_layout,
        );

        let combine_bind_group_layout =
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
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("combine_bind_group_layout"),
            });

        let combine_bind_group = make_combine_bind_group(
            &device,
            &combine_bind_group_layout,
            &processed_game_texture.1,
            &processed_game_sampler,
            &processed_gui_texture.1,
            &processed_gui_sampler,
            &egui_texture.1,
            &egui_sampler,
        );

        let combine_pipeline = combine_setup(
            &device,
            &config,
            &combine_shader(&device, resource_man),
            &combine_bind_group_layout,
        );

        Self {
            vsync,

            instance,
            device,
            queue,
            surface,
            config,
            window: Arc::new(window),

            vertex_buffer,
            index_buffer,

            game_texture,
            game_position_texture,
            game_normal_texture,
            game_depth_texture,
            processed_game_texture,
            processed_game_sampler,

            gui_texture,
            gui_position_texture,
            gui_normal_texture,
            gui_depth_texture,
            processed_gui_texture,
            processed_gui_sampler,

            egui_texture,
            egui_sampler,

            game_instance_buffer,
            game_indirect_buffer,
            game_uniform_buffer,
            game_bind_group,
            game_pipeline,

            extra_instance_buffer,
            extra_indirect_buffer,
            extra_uniform_buffer,

            effects_bind_group_layout,
            effects_noise_texture,
            effects_noise_sampler,
            game_effects_bind_group,
            gui_effects_bind_group,
            effects_pipeline,

            gui_instance_buffer,
            gui_uniform_buffer,
            gui_bind_group,
            gui_pipeline,

            overlay_vertex_buffer,
            overlay_uniform_buffer,
            overlay_bind_group,
            overlay_pipeline,

            combine_bind_group_layout,
            combine_bind_group,
            combine_pipeline,
        }
    }
}
