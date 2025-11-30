use automancy_game::persistent::options::AAType;
use wgpu::util::DeviceExt;

use crate::gpu;

#[derive(Debug)]
pub struct GameRenderTextures {
    pub albedo_texture: wgpu::TextureView,
    pub normal_texture: wgpu::TextureView,
    pub model_pos_texture: wgpu::TextureView,

    pub depth_texture: wgpu::TextureView,

    pub lighting_surface_texture: wgpu::TextureView,
    pub post_processing_surface_texture: wgpu::TextureView,
    pub fxaa_surface_texture: Option<wgpu::TextureView>,

    pub output_texture: wgpu::TextureView,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GameRenderTextures {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, antialiasing: AAType) -> Self {
        let albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Game Albedo Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::ALBEDO_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Game Normal Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::NORMAL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let model_pos_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Game Model Position Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::MODEL_POS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Game Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let surface_desc = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let lighting_surface_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Game Lighting Output Texture"),
            ..surface_desc
        });
        let post_processing_surface_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Post Processing Output Texture"),
            ..surface_desc
        });
        let fxaa_surface_texture = match antialiasing {
            AAType::None => None,
            AAType::FXAA => Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("FXAA Output Texture"),
                ..surface_desc
            })),
        };

        let output_texture = fxaa_surface_texture.clone().unwrap_or(post_processing_surface_texture.clone());

        Self {
            albedo_texture: albedo_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            normal_texture: normal_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            model_pos_texture: model_pos_texture.create_view(&wgpu::TextureViewDescriptor::default()),

            depth_texture: depth_texture.create_view(&wgpu::TextureViewDescriptor::default()),

            lighting_surface_texture: lighting_surface_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            post_processing_surface_texture: post_processing_surface_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            fxaa_surface_texture: fxaa_surface_texture.map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default())),

            output_texture: output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }
}

#[derive(Debug, Default)]
pub struct GamePipelineBuffersArgs<'a> {
    pub model_matrix_buffer: gpu::pipeline::BufferInitArg<'a>,
    pub world_matrix_buffer: gpu::pipeline::BufferInitArg<'a>,
    pub animation_matrix_buffer: gpu::pipeline::BufferInitArg<'a>,

    pub instance_buffer: gpu::pipeline::BufferInitArg<'a>,
    pub opaque_draw_buffer: Option<wgpu::Buffer>,
    pub non_opaque_draw_buffer: Option<wgpu::Buffer>,
}

#[derive(Debug)]
pub struct GamePipelineArgs<'a> {
    pub buffer: GamePipelineBuffersArgs<'a>,
}

#[derive(Debug)]
pub struct GamePipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_buffers: wgpu::BindGroup,

    pub uniform_data: gpu::data::GpuGameUniformData,
    pub uniform_buffer: wgpu::Buffer,

    pub model_matrix_buffer: wgpu::Buffer,
    pub world_matrix_buffer: wgpu::Buffer,
    pub animation_matrix_buffer: wgpu::Buffer,

    pub instance_buffer: wgpu::Buffer,
    pub opaque_draw_buffer: wgpu::Buffer,
    pub non_opaque_draw_buffer: wgpu::Buffer,
}

#[derive(Debug)]
pub struct GameLightingPipelineArgs<'a> {
    pub albedo_texture: &'a wgpu::TextureView,
    pub normal_texture: &'a wgpu::TextureView,
    pub model_pos_texture: &'a wgpu::TextureView,
}

#[derive(Debug)]
pub struct GameLightingPipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_samplers: wgpu::BindGroup,
    pub bind_group_textures: wgpu::BindGroup,

    pub uniform_data: gpu::data::GpuGameLightingUniformData,
    pub uniform_buffer: wgpu::Buffer,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GamePipeline {
    pub fn new(
        device: &wgpu::Device,
        global_res: &gpu::GlobalResources,
        GamePipelineArgs {
            buffer:
                GamePipelineBuffersArgs {
                    model_matrix_buffer,
                    world_matrix_buffer,
                    animation_matrix_buffer,

                    instance_buffer,
                    opaque_draw_buffer,
                    non_opaque_draw_buffer,
                },
        }: GamePipelineArgs,
    ) -> Self {
        let bind_group_layout_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game_bind_group_layout_uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group_layout_buffers = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game_bind_group_layout_buffers"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Game Render Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout_uniform), Some(&bind_group_layout_buffers)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Game Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.game_shader,
                entry_point: None,
                buffers: &[gpu::data::GpuGameVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.game_shader,
                entry_point: None,
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: gpu::ALBEDO_FORMAT,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: gpu::NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: gpu::MODEL_POS_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: gpu::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let uniform_data = gpu::data::GpuGameUniformData::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_bind_group_uniform"),
            layout: &bind_group_layout_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let model_matrix_buffer = match model_matrix_buffer {
            gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size,
                buffer,
                ..
            } if (min_size <= buffer.size()) => buffer,
            gpu::pipeline::BufferInitArg::Reuse(buffer) => buffer,

            gpu::pipeline::BufferInitArg::Init {
                size,
                cpu_buffer,
            }
            | gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size: size,
                cpu_buffer,
                ..
            } => gpu::util::init_buffer(
                device,
                &wgpu::BufferDescriptor {
                    label: Some("Game Model Matrix Buffer"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
                cpu_buffer,
            ),
        };

        let world_matrix_buffer = match world_matrix_buffer {
            gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size,
                buffer,
                ..
            } if (min_size <= buffer.size()) => buffer,
            gpu::pipeline::BufferInitArg::Reuse(buffer) => buffer,

            gpu::pipeline::BufferInitArg::Init {
                size,
                cpu_buffer,
            }
            | gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size: size,
                cpu_buffer,
                ..
            } => gpu::util::init_buffer(
                device,
                &wgpu::BufferDescriptor {
                    label: Some("Game World Matrix Buffer"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
                cpu_buffer,
            ),
        };

        let animation_matrix_buffer = match animation_matrix_buffer {
            gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size,
                buffer,
                ..
            } if (min_size <= buffer.size()) => buffer,
            gpu::pipeline::BufferInitArg::Reuse(buffer) => buffer,

            gpu::pipeline::BufferInitArg::Init {
                size,
                cpu_buffer,
            }
            | gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size: size,
                cpu_buffer,
                ..
            } => gpu::util::init_buffer(
                device,
                &wgpu::BufferDescriptor {
                    label: Some("Game Animation Matrix Buffer"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
                cpu_buffer,
            ),
        };

        let instance_buffer = match instance_buffer {
            gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size,
                buffer,
                ..
            } if (min_size <= buffer.size()) => buffer,
            gpu::pipeline::BufferInitArg::Reuse(buffer) => buffer,

            gpu::pipeline::BufferInitArg::Init {
                size,
                cpu_buffer,
            }
            | gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                min_size: size,
                cpu_buffer,
                ..
            } => gpu::util::init_buffer(
                device,
                &wgpu::BufferDescriptor {
                    label: Some("Game Instance Buffer"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
                cpu_buffer,
            ),
        };

        let bind_group_buffers = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_bind_group_buffers"),
            layout: &bind_group_layout_buffers,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: global_res.mesh_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: model_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: world_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: animation_matrix_buffer.as_entire_binding(),
                },
            ],
        });

        let opaque_draw_buffer = opaque_draw_buffer.unwrap_or_else(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            })
        });
        let non_opaque_draw_buffer = non_opaque_draw_buffer.unwrap_or_else(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            })
        });

        Self {
            render_pipeline,

            bind_group_uniform,
            bind_group_buffers,

            uniform_data,
            uniform_buffer,

            model_matrix_buffer,
            world_matrix_buffer,
            animation_matrix_buffer,

            instance_buffer,
            opaque_draw_buffer,
            non_opaque_draw_buffer,
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GameLightingPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &gpu::GlobalResources,
        GameLightingPipelineArgs {
            albedo_texture,
            normal_texture,
            model_pos_texture,
        }: GameLightingPipelineArgs,
    ) -> Self {
        let bind_group_layout_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game_lighting_bind_group_layout_uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group_layout_samplers = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game_lighting_bind_group_layout_samplers"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group_layout_textures = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game_lighting_bind_group_layout_textures"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false,
                        },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false,
                        },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false,
                        },
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Game Lighting Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(&bind_group_layout_uniform),
                Some(&bind_group_layout_samplers),
                Some(&bind_group_layout_textures),
            ],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Game Lighting Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.game_lighting_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.game_lighting_shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let uniform_data = gpu::data::GpuGameLightingUniformData::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Lighting Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_lighting_bind_group_uniform"),
            layout: &bind_group_layout_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let bind_group_samplers = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_lighting_bind_group_samplers"),
            layout: &bind_group_layout_samplers,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&global_res.bilinear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&global_res.point_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&global_res.repeating_sampler),
                },
            ],
        });

        let bind_group_textures = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_lighting_bind_group_textures"),
            layout: &bind_group_layout_textures,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(albedo_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(normal_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(model_pos_texture),
                },
            ],
        });

        Self {
            render_pipeline,

            bind_group_uniform,
            bind_group_samplers,
            bind_group_textures,

            uniform_data,
            uniform_buffer,
        }
    }
}
