use std::mem;

use automancy_data::math::UVec2;
use automancy_game::resources::ResourceManager;
use wgpu::util::DeviceExt;

use crate::gpu::data::{
    GpuAnimationMatrixData, GpuDrawInstance, GpuGameLightingUniformData, GpuGameUniformData, GpuModelMatrixData, GpuPostProcessingUniformData,
    GpuVertex, GpuWorldMatrixData,
};

pub mod data;
pub mod util;

pub const NORMAL_CLEAR: wgpu::Color = wgpu::Color::TRANSPARENT;
pub const MODEL_POS_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub const ALBEDO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const MODEL_POS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub const SCREENSHOT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const SCREENSHOT_PIXEL_SIZE: u32 = 4;

pub struct GameRenderTextures {
    pub albedo_texture: wgpu::Texture,
    pub normal_texture: wgpu::Texture,
    pub model_pos_texture: wgpu::Texture,

    pub depth_texture: wgpu::Texture,

    pub lighting_surface_texture: wgpu::Texture,
    pub post_processing_surface_texture: wgpu::Texture,
    pub fxaa_surface_texture: wgpu::Texture,
}

impl GameRenderTextures {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
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
            format: ALBEDO_FORMAT,
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
            format: NORMAL_FORMAT,
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
            format: MODEL_POS_FORMAT,
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
            format: DEPTH_FORMAT,
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
        let fxaa_surface_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FXAA Output Texture"),
            ..surface_desc
        });

        Self {
            albedo_texture,
            normal_texture,
            model_pos_texture,

            depth_texture,

            lighting_surface_texture,
            post_processing_surface_texture,
            fxaa_surface_texture,
        }
    }
}

pub struct GamePipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_buffers: wgpu::BindGroup,

    pub uniform_buffer: wgpu::Buffer,

    pub model_matrix_data_buffer: wgpu::Buffer,
    pub world_matrix_data_buffer: wgpu::Buffer,
    pub animation_matrix_data_buffer: wgpu::Buffer,

    pub opaque_instance_buffer: wgpu::Buffer,
    pub non_opaque_instance_buffer: wgpu::Buffer,
    pub opaque_indirect_draw_command_buffer: wgpu::Buffer,
    pub non_opaque_indirect_draw_command_buffer: wgpu::Buffer,
}

pub struct GameLightingPipelineArgs<'a> {
    pub albedo_texture: &'a wgpu::TextureView,
    pub normal_texture: &'a wgpu::TextureView,
    pub model_pos_texture: &'a wgpu::TextureView,
}
pub struct GameLightingPipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_samplers: wgpu::BindGroup,
    pub bind_group_textures: wgpu::BindGroup,

    pub uniform_buffer: wgpu::Buffer,
}

pub struct PostProcessingPipelineArgs<'a> {
    pub surface_texture: &'a wgpu::TextureView,
    pub albedo_texture: &'a wgpu::TextureView,
    pub normal_texture: &'a wgpu::TextureView,
    pub model_pos_texture: &'a wgpu::TextureView,
}

pub struct PostProcessingPipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_samplers: wgpu::BindGroup,
    pub bind_group_textures: wgpu::BindGroup,

    pub uniform_buffer: wgpu::Buffer,
}

pub struct FXAAPipelineArgs<'a> {
    pub surface_texture: &'a wgpu::TextureView,
}

pub struct FXAAPipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group: wgpu::BindGroup,
}

pub struct ComposePipelineArgs<'a> {
    pub first_texture: &'a wgpu::TextureView,
    pub first_sampler: &'a wgpu::Sampler,
    pub second_texture: &'a wgpu::TextureView,
    pub second_sampler: &'a wgpu::Sampler,
}

pub struct ComposePipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub bind_group: wgpu::BindGroup,
}

impl GamePipeline {
    pub fn new(device: &wgpu::Device, global_res: &GlobalResources, model_matrix_len: u64, world_matrix_len: u64, animation_matrix_len: u64) -> Self {
        let bind_group_layout_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_layout_uniform"),
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
            label: Some("bind_group_layout_buffers"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Game Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_uniform, &bind_group_layout_buffers],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Game Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.game_shader,
                entry_point: None,
                buffers: &[GpuVertex::desc(), GpuDrawInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.game_shader,
                entry_point: None,
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: ALBEDO_FORMAT,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: MODEL_POS_FORMAT,
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
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GpuGameUniformData::default()]),
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

        let model_matrix_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Game Model Matrix Buffer"),
            size: mem::size_of::<GpuModelMatrixData>() as u64 * model_matrix_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let world_matrix_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Game World Matrix Buffer"),
            size: mem::size_of::<GpuWorldMatrixData>() as u64 * world_matrix_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let animation_matrix_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Game Animation Matrix Buffer"),
            size: mem::size_of::<GpuAnimationMatrixData>() as u64 * animation_matrix_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_buffers = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_bind_group_buffers"),
            layout: &bind_group_layout_buffers,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_matrix_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: world_matrix_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: animation_matrix_data_buffer.as_entire_binding(),
                },
            ],
        });

        let opaque_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let non_opaque_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let opaque_indirect_draw_command_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[],
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
        });
        let non_opaque_indirect_draw_command_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[],
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            render_pipeline,

            bind_group_uniform,
            bind_group_buffers,

            uniform_buffer,

            model_matrix_data_buffer,
            world_matrix_data_buffer,
            animation_matrix_data_buffer,

            opaque_instance_buffer,
            non_opaque_instance_buffer,
            opaque_indirect_draw_command_buffer,
            non_opaque_indirect_draw_command_buffer,
        }
    }
}

impl GameLightingPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Game Lighting Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_uniform, &bind_group_layout_samplers, &bind_group_layout_textures],
            push_constant_ranges: &[],
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Lighting Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GpuGameLightingUniformData::default()]),
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

            uniform_buffer,
        }
    }
}

impl PostProcessingPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        PostProcessingPipelineArgs {
            surface_texture,
            albedo_texture,
            normal_texture,
            model_pos_texture,
        }: PostProcessingPipelineArgs,
    ) -> Self {
        let bind_group_layout_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post_processing_bind_group_layout_uniform"),
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
            label: Some("post_processing_bind_group_layout_samplers"),
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
            label: Some("post_processing_bind_group_layout_textures"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Post Processing Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_uniform, &bind_group_layout_samplers, &bind_group_layout_textures],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post Processing Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.post_processing_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.post_processing_shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Post Processing Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GpuPostProcessingUniformData::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post_processing_bind_group_uniform"),
            layout: &bind_group_layout_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let bind_group_samplers = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post_processing_bind_group_samplers"),
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
            label: Some("post_processing_bind_group_textures"),
            layout: &bind_group_layout_textures,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(surface_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(albedo_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(normal_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(model_pos_texture),
                },
            ],
        });

        Self {
            render_pipeline,

            bind_group_uniform,
            bind_group_samplers,
            bind_group_textures,

            uniform_buffer,
        }
    }
}

impl FXAAPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        FXAAPipelineArgs { surface_texture }: FXAAPipelineArgs,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fxaa_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FXAA Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FXAA Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.fxaa_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.fxaa_shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(surface_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&global_res.bilinear_sampler),
                },
            ],
        });

        Self { render_pipeline, bind_group }
    }
}

impl ComposePipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        ComposePipelineArgs {
            first_texture,
            first_sampler,
            second_texture,
            second_sampler,
        }: ComposePipelineArgs,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compose_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
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
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compose Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Compose Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_res.compose_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_res.compose_shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compose_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(first_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(first_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(second_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(second_sampler),
                },
            ],
        });

        ComposePipeline { render_pipeline, bind_group }
    }
}

pub struct GlobalResources {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub compose_shader: wgpu::ShaderModule,
    pub fxaa_shader: wgpu::ShaderModule,
    pub game_lighting_shader: wgpu::ShaderModule,
    pub game_shader: wgpu::ShaderModule,
    pub post_processing_shader: wgpu::ShaderModule,
    pub texture_sample_shader: wgpu::ShaderModule,

    pub point_sampler: wgpu::Sampler,
    pub bilinear_sampler: wgpu::Sampler,
    pub repeating_sampler: wgpu::Sampler,
}

impl GlobalResources {
    pub fn new(resource_man: &ResourceManager, device: &wgpu::Device, vertices: &[GpuVertex], indices: &[u16]) -> Self {
        let compose_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compose Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["compose"].to_string().into()),
        });

        let fxaa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FXAA Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["fxaa"].to_string().into()),
        });

        let game_lighting_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Lighting Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["game_lighting"].to_string().into()),
        });

        let game_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["game"].to_string().into()),
        });

        let post_processing_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Processing Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["post_processing"].to_string().into()),
        });

        let texture_sample_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Sample Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["texture_sample"].to_string().into()),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let point_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bilinear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let repeating_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            vertex_buffer,
            index_buffer,

            compose_shader,
            fxaa_shader,
            game_lighting_shader,
            game_shader,
            post_processing_shader,
            texture_sample_shader,

            point_sampler,
            bilinear_sampler,
            repeating_sampler,
        }
    }
}

pub struct GameRenderResources {
    pub render_textures: GameRenderTextures,

    pub game_pipeline: GamePipeline,
    pub game_lighting_pipeline: GameLightingPipeline,
    pub post_processing_pipeline: PostProcessingPipeline,
    pub fxaa_pipeline: FXAAPipeline,
}

impl GameRenderResources {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, global_res: &GlobalResources) -> Self {
        let render_textures = GameRenderTextures::new(device, config);

        let albedo_texture = render_textures.albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_texture = render_textures.normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let model_pos_texture = render_textures.model_pos_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let lighting_surface_texture = render_textures
            .lighting_surface_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let post_processing_surface_texture = render_textures
            .post_processing_surface_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let _fxaa_surface_texture = render_textures.fxaa_surface_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let game_pipeline = GamePipeline::new(device, global_res, 2048, 1, 2048);
        let game_lighting_pipeline = GameLightingPipeline::new(
            device,
            config,
            global_res,
            GameLightingPipelineArgs {
                albedo_texture: &albedo_texture,
                normal_texture: &normal_texture,
                model_pos_texture: &model_pos_texture,
            },
        );
        let post_processing_pipeline = PostProcessingPipeline::new(
            device,
            config,
            global_res,
            PostProcessingPipelineArgs {
                surface_texture: &lighting_surface_texture,
                albedo_texture: &albedo_texture,
                normal_texture: &normal_texture,
                model_pos_texture: &model_pos_texture,
            },
        );
        let fxaa_pipeline = FXAAPipeline::new(
            device,
            config,
            global_res,
            FXAAPipelineArgs {
                surface_texture: &post_processing_surface_texture,
            },
        );

        Self {
            render_textures,
            game_pipeline,
            game_lighting_pipeline,
            post_processing_pipeline,
            fxaa_pipeline,
        }
    }
}

pub struct GuiRenderResources {
    pub gui_texture: wgpu::Texture,
}

impl GuiRenderResources {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, _global_res: &GlobalResources) -> Self {
        let gui_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gui Output Texture"),
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
        });

        Self { gui_texture }
    }
}

pub struct PresentResources {
    pub game_gui_compose_pipeline: ComposePipeline,

    pub present_texture: wgpu::Texture,
}

impl PresentResources {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        game_res: &GameRenderResources,
        gui_res: &GuiRenderResources,
    ) -> Self {
        let game_gui_compose_pipeline = ComposePipeline::new(
            device,
            config,
            global_res,
            ComposePipelineArgs {
                first_texture: &game_res
                    .render_textures
                    .fxaa_surface_texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                first_sampler: &global_res.point_sampler,
                second_texture: &gui_res.gui_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                second_sampler: &global_res.point_sampler,
            },
        );

        let present_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Present Texture"),
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
        });

        Self {
            game_gui_compose_pipeline,
            present_texture,
        }
    }
}

pub struct RenderResources {
    pub adapter_info: wgpu::AdapterInfo,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,

    pub global_res: GlobalResources,
    pub main_game_res: GameRenderResources,
    pub gui_res: GuiRenderResources,
    pub present_res: PresentResources,

    vsync: bool,
}

impl RenderResources {
    fn pick_present_mode(vsync: bool) -> wgpu::PresentMode {
        if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        }
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        if self.vsync != vsync {
            self.vsync = vsync;
            self.config.present_mode = Self::pick_present_mode(vsync);

            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn resize(&mut self, size: UVec2) {
        self.config.width = size.x;
        self.config.height = size.y;

        self.surface.configure(&self.device, &self.config);

        self.main_game_res = GameRenderResources::new(&self.device, &self.config, &self.global_res);
        self.gui_res = GuiRenderResources::new(&self.device, &self.config, &self.global_res);
        self.present_res = PresentResources::new(&self.device, &self.config, &self.global_res, &self.main_game_res, &self.gui_res);
    }

    pub async fn new(
        resource_man: &ResourceManager,
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        vertices: &[GpuVertex],
        indices: &[u16],
    ) -> Self {
        let vsync = true;

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all()),
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::HighPerformance),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::INDIRECT_FIRST_INSTANCE,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                label: None,
                trace: Default::default(),
            })
            .await
            .unwrap();

        let global_res = GlobalResources::new(resource_man, &device, vertices, indices);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        log::info!("Surface format: {surface_format:?}");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 1,
            height: 1,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let main_game_res = GameRenderResources::new(&device, &config, &global_res);
        let gui_res = GuiRenderResources::new(&device, &config, &global_res);
        let present_res = PresentResources::new(&device, &config, &global_res, &main_game_res, &gui_res);

        RenderResources {
            adapter_info: adapter.get_info(),
            instance,
            device,
            queue,
            surface,
            config,

            global_res,
            main_game_res,
            gui_res,
            present_res,

            vsync,
        }
    }
}
