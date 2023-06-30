use egui::{pos2, Rect};
use slice_group_by::GroupBy;
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirect};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device,
    DeviceDescriptor, Extent3d, Features, FragmentState, FrontFace, Instance, InstanceDescriptor,
    Limits, MultisampleState, PipelineLayoutDescriptor, PowerPreference, PrimitiveState,
    PrimitiveTopology, Queue, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderStages, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexState,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use automancy_defs::cg::{Double, Float};
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameUBO, OverlayUBO, RawInstanceData, Vertex};
use automancy_defs::{bytemuck, shaders};
use automancy_resources::ResourceManager;

pub const GPU_BACKENDS: Backends = Backends::all();

pub fn device_descriptor() -> DeviceDescriptor<'static> {
    DeviceDescriptor {
        features: Features::INDIRECT_FIRST_INSTANCE
            | Features::MULTI_DRAW_INDIRECT
            | Features::DEPTH_CLIP_CONTROL,
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

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub const MULTISAMPLE_SURFACE_TEXTURE: Option<&str> = Some("Multisample Surface Texture");

pub const GAME_DEPTH_TEXTURE: Option<&str> = Some("Game Depth Texture");
pub const GAME_INDIRECT_BUFFER: Option<&str> = Some("Game Indirect Buffer");
pub const GAME_INSTANCE_BUFFER: Option<&str> = Some("Game Instance Buffer");

pub const GUI_DEPTH_TEXTURE: Option<&str> = Some("Gui Depth Texture");
pub const GUI_INSTANCE_BUFFER: Option<&str> = Some("Gui Instance Buffer");

pub const OVERLAY_VERTEX_BUFFER: Option<&str> = Some("Overlay Vertex Buffer");

// TODO use these!
// TODO move to window::

pub fn window_size_rect(window: &Window) -> Rect {
    let (width, height) = window_size_float(window);

    Rect::from_min_max(pos2(0.0, 0.0), pos2(width, height))
}

pub fn window_size_double(window: &Window) -> (Double, Double) {
    window.inner_size().cast::<Double>().into()
}

pub fn window_size_float(window: &Window) -> (Float, Float) {
    window.inner_size().cast::<Float>().into()
}

pub fn window_size_u32(window: &Window) -> (u32, u32) {
    window.inner_size().into()
}

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

            let index_range = resource_man.index_ranges[&resource_man.get_model(ids[0])];

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
    config: &SurfaceConfiguration,
    label: Option<&str>,
) -> (Texture, TextureView) {
    let size = Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 4,
        dimension: TextureDimension::D2,
        format: config.format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

pub fn create_depth_texture(
    device: &Device,
    config: &SurfaceConfiguration,
    label: Option<&str>,
) -> (Texture, TextureView) {
    let size = Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 4,
        dimension: TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

pub struct Gpu {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub config: SurfaceConfiguration,
    pub window: Window,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    pub multisample_surface_texture: (Texture, TextureView),

    pub game_depth_texture: (Texture, TextureView),
    pub game_instance_buffer: Buffer,
    pub game_indirect_buffer: Buffer,
    pub game_uniform_buffer: Buffer,
    pub game_bind_group: BindGroup,
    pub game_pipeline: RenderPipeline,

    pub gui_depth_texture: (Texture, TextureView),
    pub gui_instance_buffer: Buffer,
    pub gui_uniform_buffer: Buffer,
    pub gui_bind_group: BindGroup,
    pub gui_pipeline: RenderPipeline,

    pub overlay_vertex_buffer: Buffer,
    pub overlay_uniform_buffer: Buffer,
    pub overlay_bind_group: BindGroup,
    pub overlay_pipeline: RenderPipeline,
}

impl Gpu {
    fn game_setup(
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> (Buffer, BindGroup, RenderPipeline) {
        let shader = shaders::game_shader(device);

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
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), RawInstanceData::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
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
                unclipped_depth: true,
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
                count: 4,
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
    ) -> (Buffer, BindGroup, RenderPipeline) {
        let shader = shaders::game_shader(device);

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
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), RawInstanceData::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
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
                unclipped_depth: true,
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
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        (uniform_buffer, bind_group, pipeline)
    }

    fn overlay_setup(
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> (Buffer, BindGroup, RenderPipeline) {
        let shader = shaders::overlay_shader(device);

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
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
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
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        (uniform_buffer, bind_group, pipeline)
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;

        self.game_depth_texture =
            create_depth_texture(&self.device, &self.config, GAME_DEPTH_TEXTURE);
        self.gui_depth_texture =
            create_depth_texture(&self.device, &self.config, GUI_DEPTH_TEXTURE);
        self.multisample_surface_texture =
            create_surface_texture(&self.device, &self.config, MULTISAMPLE_SURFACE_TEXTURE);

        self.surface.configure(&self.device, &self.config);
    }

    pub async fn new(window: Window, vertices: Vec<Vertex>, indices: Vec<u16>) -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor {
            backends: GPU_BACKENDS,
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
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
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

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

        let game_depth_texture = create_depth_texture(&device, &config, GAME_DEPTH_TEXTURE);
        let multisample_surface_texture =
            create_surface_texture(&device, &config, MULTISAMPLE_SURFACE_TEXTURE);

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

        let (game_uniform_buffer, game_bind_group, game_pipeline) =
            Self::game_setup(&device, &config);

        let gui_depth_texture = create_depth_texture(&device, &config, GUI_DEPTH_TEXTURE);

        let gui_instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: GUI_INSTANCE_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let (gui_uniform_buffer, gui_bind_group, gui_pipeline) = Self::gui_setup(&device, &config);

        let overlay_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: OVERLAY_VERTEX_BUFFER,
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let (overlay_uniform_buffer, overlay_bind_group, overlay_pipeline) =
            Self::overlay_setup(&device, &config);

        Self {
            instance,
            device,
            queue,
            surface,
            config,
            window,

            vertex_buffer,
            index_buffer,

            multisample_surface_texture,
            game_depth_texture,
            game_instance_buffer,
            game_indirect_buffer,
            game_uniform_buffer,
            game_bind_group,
            game_pipeline,

            gui_depth_texture,
            gui_instance_buffer,
            gui_uniform_buffer,
            gui_bind_group,
            gui_pipeline,

            overlay_vertex_buffer,
            overlay_uniform_buffer,
            overlay_bind_group,
            overlay_pipeline,
        }
    }
}
