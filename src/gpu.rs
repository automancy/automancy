use std::rc::Rc;
use std::sync::Arc;

use egui_wgpu::wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirect};
use egui_wgpu::wgpu::{
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
use automancy_defs::hashbrown::{HashMap, HashSet};
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameUBO, InstanceData, PostEffectsUBO, RawInstanceData, Vertex};
use automancy_defs::slice_group_by::GroupBy;
use automancy_macros::OptionGetter;
use automancy_resources::ResourceManager;

pub const GPU_BACKENDS: Backends = Backends::all();

pub const NORMAL_CLEAR: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const SCREENSHOT_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub fn compile_instances<T: Clone>(
    resource_man: &ResourceManager,
    instances: &[(InstanceData, Id, T)],
) -> HashMap<Id, Vec<(usize, RawInstanceData, T)>> {
    let mut raw_instances = HashMap::new();

    #[cfg(debug_assertions)]
    let mut seen = HashSet::new();

    instances.binary_group_by_key(|v| v.1).for_each(|v| {
        let id = v[0].1;

        #[cfg(debug_assertions)]
        {
            if seen.contains(&id) {
                panic!("Duplicate id when collecting instances - are the instances sorted?");
            }
            seen.insert(id);
        }

        let models = &resource_man.all_models[&id].0;

        for (instance, _, extra) in v.iter() {
            models.values().for_each(|model| {
                raw_instances.entry(id).or_insert_with(Vec::new).push((
                    model.index,
                    RawInstanceData::from(*instance),
                    extra.clone(),
                ));
            });
        }
    });

    raw_instances
        .values_mut()
        .for_each(|v| v.sort_by_key(|v| v.0));

    raw_instances
}

pub fn indirect_instance<T: Clone>(
    resource_man: &ResourceManager,
    instances: &[(InstanceData, Id, T)],
    group: bool,
) -> (
    Vec<RawInstanceData>,
    HashMap<Id, Vec<(DrawIndexedIndirect, T)>>,
    u32,
) {
    let raw_instances = compile_instances(resource_man, instances);

    let mut base_instance_counter = 0;
    let mut indirect_commands = HashMap::new();
    let mut draw_count = 0;

    raw_instances.iter().for_each(|(id, instances)| {
        if group {
            instances
                .exponential_group_by_key(|v| v.0)
                .for_each(|instances| {
                    let size = instances.len() as u32;
                    let index_range = resource_man.all_index_ranges[id][&instances[0].0];

                    let command = DrawIndexedIndirect {
                        base_index: index_range.offset,
                        vertex_offset: 0,
                        vertex_count: index_range.size,
                        base_instance: base_instance_counter,
                        instance_count: size,
                    };

                    base_instance_counter += size;
                    draw_count += 1;

                    indirect_commands
                        .entry(*id)
                        .or_insert_with(Vec::new)
                        .push((command, instances[0].2.clone()));
                });
        } else {
            //TODO dedupe these code
            instances.iter().for_each(|instance| {
                let size = 1;
                let index_range = resource_man.all_index_ranges[id][&instance.0];

                let command = DrawIndexedIndirect {
                    base_index: index_range.offset,
                    vertex_offset: 0,
                    vertex_count: index_range.size,
                    base_instance: base_instance_counter,
                    instance_count: size,
                };

                base_instance_counter += size;
                draw_count += 1;

                indirect_commands
                    .entry(*id)
                    .or_insert_with(Vec::new)
                    .push((command, instance.2.clone()));
            });
        }
    });

    let raw_instances = raw_instances
        .into_iter()
        .flat_map(|v| v.1.into_iter().map(|v| v.1))
        .collect::<Vec<_>>();

    (raw_instances, indirect_commands, draw_count)
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

pub fn create_texture_and_view(
    device: &Device,
    descriptor: &TextureDescriptor,
) -> (Texture, TextureView) {
    let texture = device.create_texture(descriptor);

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
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

fn make_antialiasing_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    texture: &TextureView,
    sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(texture),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
        ],
        label: Some("antialiasing_bind_group"),
    })
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
    #[getters(get)]
    antialiasing_bind_group: Option<BindGroup>,
}

#[derive(OptionGetter)]
pub struct InWorldItemResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
}

#[derive(OptionGetter)]
pub struct GuiResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub post_effects_uniform_buffer: Buffer,
    #[getters(get)]
    post_effects_bind_group: Option<BindGroup>,
    #[getters(get)]
    antialiasing_bind_group: Option<BindGroup>,
}

#[derive(OptionGetter)]
pub struct ItemResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
}

#[derive(OptionGetter)]
pub struct EguiResources {
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
}

#[derive(OptionGetter)]
pub struct OverlayResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
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
pub struct AntialiasingResources {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
}

#[derive(OptionGetter)]
pub struct IntermediateResources {
    pub bind_group_layout: BindGroupLayout,
    pub screenshot_pipeline: RenderPipeline,
    pub present_pipeline: RenderPipeline,
    #[getters(get)]
    present_bind_group: Option<BindGroup>,
}

pub struct Gpu {
    vsync: bool,

    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub config: SurfaceConfiguration,
    pub window: Arc<Window>,

    pub game_shader: ShaderModule,
    pub post_effects_shader: ShaderModule,
    pub combine_shader: ShaderModule,
    pub intermediate_shader: ShaderModule,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    game_texture: Option<(Texture, TextureView)>,
    normal_texture: Option<(Texture, TextureView)>,
    depth_texture: Option<(Texture, TextureView)>,
    model_depth_texture: Option<(Texture, TextureView)>,

    pub filtering_sampler: Sampler,
    pub non_filtering_sampler: Sampler,

    pub game_resources: GameResources,
    pub in_world_item_resources: InWorldItemResources,
    pub gui_resources: GuiResources,
    pub item_resources: ItemResources,
    pub egui_resources: EguiResources,
    pub overlay_resources: OverlayResources,
    pub first_combine_resources: CombineResources,
    pub post_effects_resources: PostEffectsResources,
    pub second_combine_resources: CombineResources,
    pub antialiasing_resources: AntialiasingResources,
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

    pub fn model_depth_texture(&self) -> &(Texture, TextureView) {
        self.model_depth_texture.as_ref().unwrap()
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
            .request_device(
                &DeviceDescriptor {
                    features: Features::INDIRECT_FIRST_INSTANCE | Features::MULTI_DRAW_INDIRECT,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        Limits::downlevel_webgl2_defaults()
                    } else {
                        Limits::default()
                    },
                    label: None,
                },
                None,
            )
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
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let game_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["game"].as_str().into()),
        });

        let post_effects_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Post Effects Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["post_effects"].as_str().into()),
        });

        let combine_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Combine Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["combine"].as_str().into()),
        });

        let antialiasing_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Antialiasing Shader"),
            source: ShaderSource::Wgsl(resource_man.shaders["antialiasing"].as_str().into()),
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
                    module: &game_shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc(), RawInstanceData::desc()],
                },
                fragment: Some(FragmentState {
                    module: &game_shader,
                    entry_point: "fs_main",
                    targets: &[
                        Some(ColorTargetState {
                            format: config.format,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::ALL,
                        }),
                        Some(ColorTargetState {
                            format: TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        }),
                        Some(ColorTargetState {
                            format: TextureFormat::R32Float,
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
                antialiasing_bind_group: None,
            }
        };

        let in_world_item_resources = {
            let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("In-world Item Uniform Buffer"),
                contents: bytemuck::cast_slice(&[GameUBO::default()]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            InWorldItemResources {
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

        let gui_resources = {
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

            GuiResources {
                instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
                bind_group,
                post_effects_uniform_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Gui Post Effects Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[PostEffectsUBO::default()]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }),
                post_effects_bind_group: None,
                antialiasing_bind_group: None,
            }
        };

        let item_resources = {
            let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Item Uniform Buffer"),
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
                label: Some("item_bind_group_layout"),
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
                label: Some("item_bind_group"),
            });

            ItemResources {
                instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &[],
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }),
                uniform_buffer,
                bind_group,
            }
        };

        let egui_resources = EguiResources { texture: None };

        let overlay_resources = {
            let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Overlay Uniform Buffer"),
                contents: bytemuck::cast_slice(&[GameUBO::default()]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            OverlayResources {
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

        let combine_bind_group_layout =
            Rc::new(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            }));

        let combine_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Combine Render Pipeline Layout"),
            bind_group_layouts: &[&combine_bind_group_layout],
            push_constant_ranges: &[],
        });

        let combine_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Combine Render Pipeline"),
            layout: Some(&combine_pipeline_layout),
            vertex: VertexState {
                module: &combine_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &combine_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: None,
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

        let combine_pipeline = Rc::new(combine_pipeline);

        let first_combine_resources = CombineResources {
            bind_group_layout: combine_bind_group_layout.clone(),
            pipeline: combine_pipeline.clone(),
            bind_group: None,
            texture: None,
        };

        let post_effects_resources = {
            let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            });

            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Post Effects Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Post Effects Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &post_effects_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &post_effects_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: None,
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

        let antialiasing_resources = {
            let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                ],
                label: Some("antialiasing_bind_group_layout"),
            });

            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Antialiasing Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Antialiasing Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &antialiasing_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &antialiasing_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: None,
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

            AntialiasingResources {
                bind_group_layout,
                pipeline,
                texture: None,
            }
        };

        let intermediate_resources = {
            let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
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
                        blend: None,
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

            let present_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Present Pipeline"),
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
                        format: config.format,
                        blend: None,
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
                present_pipeline,
                present_bind_group: None,
            }
        };

        let mut this = Self {
            vsync: false,

            instance,
            device,
            queue,
            surface,
            config,
            window: Arc::new(window),

            game_shader,
            post_effects_shader,
            combine_shader,
            intermediate_shader,

            vertex_buffer,
            index_buffer,

            game_texture: None,
            normal_texture: None,
            depth_texture: None,
            model_depth_texture: None,

            filtering_sampler,
            non_filtering_sampler,

            game_resources,
            in_world_item_resources,
            gui_resources,
            item_resources,
            egui_resources,
            overlay_resources,
            first_combine_resources,
            post_effects_resources,
            second_combine_resources,
            antialiasing_resources,
            intermediate_resources,
        };

        this.create_textures(size);

        this
    }

    pub fn create_textures(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;

        let device = &self.device;
        let config = &self.config;

        self.surface.configure(device, config);

        let extent = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        self.game_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.normal_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.depth_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        ));
        self.model_depth_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.egui_resources.texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.game_resources.post_effects_bind_group = Some(make_post_effects_bind_group(
            device,
            &self.post_effects_resources.bind_group_layout,
            &self.game_resources.post_effects_uniform_buffer,
            &self.game_texture().1,
            &self.non_filtering_sampler,
            &self.normal_texture().1,
            &self.non_filtering_sampler,
            &self.model_depth_texture().1,
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
            &self.model_depth_texture().1,
            &self.non_filtering_sampler,
        ));

        self.post_effects_resources.texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.antialiasing_resources.texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.game_resources.antialiasing_bind_group = Some(make_antialiasing_bind_group(
            device,
            &self.antialiasing_resources.bind_group_layout,
            &self.post_effects_resources.texture().1,
            &self.filtering_sampler,
        ));
        self.gui_resources.antialiasing_bind_group = Some(make_antialiasing_bind_group(
            device,
            &self.antialiasing_resources.bind_group_layout,
            &self.post_effects_resources.texture().1,
            &self.filtering_sampler,
        ));

        self.first_combine_resources.texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.first_combine_resources.bind_group = Some(make_combine_bind_group(
            device,
            &self.first_combine_resources.bind_group_layout,
            &self.antialiasing_resources.texture().1,
            &self.non_filtering_sampler,
            &self.egui_resources.texture().1,
            &self.non_filtering_sampler,
        ));

        self.second_combine_resources.texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.second_combine_resources.bind_group = Some(make_combine_bind_group(
            device,
            &self.second_combine_resources.bind_group_layout,
            &self.first_combine_resources.texture().1,
            &self.non_filtering_sampler,
            &self.antialiasing_resources.texture().1,
            &self.non_filtering_sampler,
        ));

        self.intermediate_resources.present_bind_group =
            Some(device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.intermediate_resources.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &self.second_combine_resources.texture().1,
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.non_filtering_sampler),
                    },
                ],
            }))
    }
}
