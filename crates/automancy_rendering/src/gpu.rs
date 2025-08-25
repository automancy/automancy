use std::{mem, num::NonZero, sync::Arc};

use automancy_data::rendering::gpu::PostProcessingUniformData;
use automancy_game::resources::ResourceManager;
use bytemuck::Pod;
use ordermap::OrderMap;
use slice_group_by::GroupBy;
use winit::{dpi::PhysicalSize, window::Window};

use crate::data::{
    GpuAnimationMatrixData, GpuDrawInstance, GpuGameMatrixData, GpuGameUniformData, GpuVertex,
    GpuWorldMatrixData,
};

pub const NORMAL_CLEAR: wgpu::Color = wgpu::Color::TRANSPARENT;
pub const MODEL_DEPTH_CLEAR: wgpu::Color = wgpu::Color {
    r: -1.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MODEL_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
pub const SCREENSHOT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

fn align_to_copy_alignment(add: wgpu::BufferAddress) -> wgpu::BufferAddress {
    add + (wgpu::COPY_BUFFER_ALIGNMENT - (add % wgpu::COPY_BUFFER_ALIGNMENT))
}

fn ordered_map_write_to_buffer<K, V>(data: &OrderMap<K, V>) -> Vec<u8>
where
    V: Pod + Default,
{
    let mut init_buffer = vec![];

    for i in 0..data.len() {
        init_buffer.extend_from_slice(bytemuck::bytes_of(
            &data.get_index(i).map(|v| *v.1).unwrap_or_default(),
        ));
    }

    init_buffer
}

pub fn ordered_map_update_buffer<K, V>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    data: &OrderMap<K, V>,
) where
    V: Pod + Default,
{
    queue.write_buffer(buffer, 0, &ordered_map_write_to_buffer(data));
}

#[must_use]
pub fn update_buffer_with_changes<V>(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    changes: &[usize],
    data: &[V],
) -> Option<wgpu::util::StagingBelt>
where
    V: Pod + Default,
{
    debug_assert!(changes.windows(2).all(|v| v[0] < v[1]));
    if data.is_empty() {
        return None;
    }

    let byte_size = size_of::<V>();

    let entire_size = changes.len();
    if let Some(max_batch) = changes
        .linear_group_by(|a, b| b - a == 1)
        .map(|v| v.len())
        .max()
    {
        // TODO test a good multiplier for the size
        let mut belt = wgpu::util::StagingBelt::new(align_to_copy_alignment(
            (byte_size * entire_size / 4).max(byte_size * max_batch) as wgpu::BufferAddress,
        ));

        for batch in changes.linear_group_by(|a, b| b - a == 1) {
            if !batch.is_empty() {
                let start = batch[0];
                if start >= data.len() {
                    continue;
                }
                let end = (start + batch.len()).min(data.len());

                let size = end - start;

                let mut view = belt.write_buffer(
                    encoder,
                    buffer,
                    (byte_size * start) as wgpu::BufferAddress,
                    unsafe { NonZero::new_unchecked((byte_size * size) as wgpu::BufferAddress) },
                    device,
                );

                view.copy_from_slice(bytemuck::cast_slice(&data[start..end]))
            }
        }

        belt.finish();

        return Some(belt);
    }

    None
}

#[must_use]
pub fn resize_update_buffer_with_changes<V>(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    buffer: &mut wgpu::Buffer,
    changes: &[usize],
    data: &[V],
) -> Option<wgpu::util::StagingBelt>
where
    V: Pod + Default,
{
    debug_assert!(changes.windows(2).all(|v| v[0] < v[1]));

    let byte_size = size_of::<V>();

    let max_index = changes.last().cloned().unwrap_or_default();
    let size = max_index + 1;

    if (buffer.size() as usize) < byte_size * size {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage,
        });
    } else {
        return update_buffer_with_changes(encoder, device, buffer, changes, data);
    }

    None
}

pub fn resize_update_buffer<V>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    data: &[V],
) where
    V: Pod,
{
    if (buffer.size() as usize) < std::mem::size_of_val(data) {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage,
        });
    } else {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }
}

pub fn clear_buffer(device: &wgpu::Device, buffer: &mut wgpu::Buffer) {
    let usage = buffer.usage();

    *buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[],
        usage,
    });
}

pub struct GamePipelineArgs {
    pub albedo_texture: wgpu::TextureView,
    pub normal_texture: wgpu::TextureView,
    pub depth_texture: wgpu::TextureView,
    pub viewspace_depth_texture: wgpu::TextureView,
}

pub struct GamePipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_buffers: wgpu::BindGroup,

    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub matrix_data_buffer: wgpu::Buffer,
    pub animation_matrix_data_buffer: wgpu::Buffer,
    pub world_matrix_data_buffer: wgpu::Buffer,
}

pub struct PostProcessingPipelineArgs {
    pub albedo_texture: wgpu::TextureView,
    pub normal_texture: wgpu::TextureView,
    pub depth_texture: wgpu::TextureView,
}

pub struct PostProcessingPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_uniform: wgpu::BindGroup,
    pub bind_group_textures: wgpu::BindGroup,

    pub uniform_buffer: wgpu::Buffer,
}

pub struct FXAAPipelineArgs {
    pub surface_texture: wgpu::TextureView,
}

pub struct FXAAPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
}

pub struct ComposePipelineArgs {
    pub first_texture: wgpu::TextureView,
    pub first_sampler: wgpu::Sampler,
    pub second_texture: wgpu::TextureView,
    pub second_sampler: wgpu::Sampler,
}

pub struct ComposePipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
}

impl GamePipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
        GamePipelineArgs {
            albedo_texture,
            normal_texture,
            depth_texture,
            viewspace_depth_texture,
        }: &GamePipelineArgs,
    ) -> Self {
        let bind_group_buffers_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("game_bind_group_buffers_layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Game Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_buffers_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Game Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_resources.game_shader,
                entry_point: None,
                buffers: &[GpuVertex::desc(), GpuDrawInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_resources.game_shader,
                entry_point: None,
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: MODEL_DEPTH_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
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
                depth_compare: wgpu::CompareFunction::Less,
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

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GpuGameUniformData::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let matrix_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<GpuGameMatrixData>() * 524288],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let animation_matrix_data_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Game Animation Matrix Data Buffer"),
                contents: &vec![0; mem::size_of::<GpuAnimationMatrixData>() * 524288],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let world_matrix_data_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Game World Matrix Data Buffer"),
                contents: &vec![0; mem::size_of::<GpuWorldMatrixData>()],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_buffers = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_buffers_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: animation_matrix_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: world_matrix_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("game_bind_group"),
        });

        Self {
            render_pipeline,
            bind_group_buffers,

            instance_buffer,
            matrix_data_buffer,
            animation_matrix_data_buffer,
            world_matrix_data_buffer,
            uniform_buffer,
        }
    }
}

impl PostProcessingPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
        PostProcessingPipelineArgs {
            albedo_texture,
            normal_texture,
            depth_texture,
        }: &PostProcessingPipelineArgs,
    ) -> Self {
        let bind_group_layout_uniform =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("post_processing_bind_group_layout_uniform"),
            });

        let bind_group_layout_textures =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                ],
                label: Some("post_processing_bind_group_layout_textures"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Post Processing Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout_textures, &bind_group_layout_uniform],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post Processing Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_resources.post_processing_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_resources.post_processing_shader,
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
            contents: bytemuck::cast_slice(&[PostProcessingUniformData::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let bind_group_textures = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &global_resources.post_processing_bind_group_layout_textures,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&global_resources.bilinear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&global_resources.point_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&global_resources.repeating_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(albedo_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(normal_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(depth_texture),
                },
            ],
            label: None,
        }));

        Self {
            render_pipeline,
            bind_group_uniform,
            bind_group_textures,
            uniform_buffer,
        }
    }
}

impl FXAAPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
        FXAAPipelineArgs { surface_texture }: &FXAAPipelineArgs,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("fxaa_bind_group_layout"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("FXAA Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FXAA Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_resources.fxaa_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_resources.fxaa_shader,
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
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(surface_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(global_resources.point_sampler),
                },
            ],
            label: Some("fxaa_bind_group"),
        });

        Self {
            render_pipeline,
            bind_group,
        }
    }
}

impl ComposePipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
        ComposePipelineArgs {
            first_texture,
            first_sampler,
            second_texture,
            second_sampler,
        }: &ComposePipelineArgs,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("compose_bind_group_layout"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compose Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Compose Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &global_resources.compose_shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &global_resources.compose_shader,
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
            layout: bind_group_layout,
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
            label: Some("compose_bind_group"),
        });
    }
}

pub struct GlobalResources {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub compose_shader: wgpu::ShaderModule,
    pub fxaa_shader: wgpu::ShaderModule,
    pub game_shader: wgpu::ShaderModule,
    pub intermediate_shader: wgpu::ShaderModule,
    pub post_processing_shader: wgpu::ShaderModule,

    pub point_sampler: wgpu::Sampler,
    pub bilinear_sampler: wgpu::Sampler,
    pub repeating_sampler: wgpu::Sampler,
}

impl GlobalResources {
    pub fn new(
        resource_man: &ResourceManager,
        device: &wgpu::Device,
        vertices: Vec<GpuVertex>,
        indices: Vec<u16>,
    ) -> Self {
        let compose_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compose Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["compose"].to_string().into()),
        });

        let fxaa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FXAA Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["fxaa"].to_string().into()),
        });

        let game_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["game"].to_string().into()),
        });

        let intermediate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Intermediate Shader"),
            source: wgpu::ShaderSource::Wgsl(
                resource_man.shaders["intermediate"].to_string().into(),
            ),
        });

        let post_processing_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Processing Shader"),
            source: wgpu::ShaderSource::Wgsl(
                resource_man.shaders["post_processing"].to_string().into(),
            ),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });

        let point_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bilinear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let repeating_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            vertex_buffer,
            index_buffer,

            compose_shader,
            fxaa_shader,
            game_shader,
            intermediate_shader,
            post_processing_shader,

            point_sampler,
            bilinear_sampler,
            repeating_sampler,
        }
    }
}

pub struct RenderTextures {
    pub albedo_texture: wgpu::Texture,
    pub normal_texture: wgpu::Texture,
    pub depth_texture: wgpu::Texture,

    pub viewspace_depth_texture: wgpu::Texture,

    pub surface_texture: wgpu::Texture,
}

impl RenderTextures {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
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
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: MODEL_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let viewspace_depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
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

        let surface_texture = device.create_texture(&wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        Self {
            albedo_texture,
            normal_texture,
            depth_texture,

            viewspace_depth_texture,

            surface_texture,
        }
    }
}

/*
impl SharedResources {
    pub fn create(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
    ) {
        let extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };


        self.game_antialiasing_bind_group = Some(make_fxaa_bind_group(
            device,
            &global_resources.fxaa_bind_group_layout,
            &self.game_post_processing_texture().1,
            &global_resources.filtering_sampler,
        ));
        self.game_antialiasing_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: config.width,
                    height: config.height,
                    ..Default::default()
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.overlay_depth_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.first_combine_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: config.width,
                    height: config.height,
                    ..Default::default()
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.first_combine_bind_group = Some(make_combine_bind_group(
            device,
            &global_resources.combine_bind_group_layout,
            &self.game_antialiasing_texture().1,
            &global_resources.filtering_sampler,
            &self.gui_texture_resolve().1,
            &global_resources.filtering_sampler,
        ));

        self.present_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &global_resources.intermediate_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.first_combine_texture().1),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&global_resources.nonfiltering_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: global_resources.present_uniform_buffer.as_entire_binding(),
                },
            ],
        }));
        self.screenshot_bind_group = Some(
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &global_resources.intermediate_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&self.first_combine_texture().1),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&global_resources.nonfiltering_sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: global_resources
                            .screenshot_uniform_buffer
                            .as_entire_binding(),
                    },
                ],
            }),
        );
    }
}
 */

pub struct GameRenderResources {
    pub render_textures: RenderTextures,

    pub game_pipeline: GamePipeline,
    pub post_processing_pipeline: PostProcessingPipeline,
    pub fxaa_pipeline: FXAAPipeline,
}

impl GameRenderResources {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_resources: &GlobalResources,
    ) -> Self {
        let render_textures = RenderTextures::new(&device, &config);

        let albedo_texture = render_textures
            .albedo_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let normal_texture = render_textures
            .normal_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture = render_textures
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let viewspace_depth_texture = render_textures
            .viewspace_depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let surface_texture = render_textures
            .surface_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let game_pipeline = GamePipeline::new(
            device,
            config,
            global_resources,
            &GamePipelineArgs {
                albedo_texture,
                normal_texture,
                depth_texture,

                viewspace_depth_texture,
            },
        );
        let post_processing_pipeline = PostProcessingPipeline::new(
            device,
            config,
            global_resources,
            &PostProcessingPipelineArgs {
                albedo_texture,
                normal_texture,
                depth_texture,
            },
        );
        let fxaa_pipeline = FXAAPipeline::new(
            device,
            config,
            global_resources,
            &&FXAAPipelineArgs { surface_texture },
        );

        Self {
            render_textures,
            game_pipeline,
            post_processing_pipeline,
            fxaa_pipeline,
        }
    }
}

pub struct AutomancyRenderResources {
    pub window: Arc<Window>,

    pub adapter_info: wgpu::AdapterInfo,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,

    pub global_resources: GlobalResources,
    pub game_render_resources: GameRenderResources,

    vsync: bool,
}

impl AutomancyRenderResources {
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;

        self.surface.configure(&self.device, &self.config);
        self.game_render_resources =
            GameRenderResources::new(&self.device, &self.config, &self.global_resources);
    }

    pub async fn new(
        resource_man: &ResourceManager,
        vertices: Vec<GpuVertex>,
        indices: Vec<u16>,
        window: Arc<Window>,
        vsync: bool,
    ) -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all()),
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::from_env()
                    .unwrap_or(wgpu::PowerPreference::HighPerformance),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::INDIRECT_FIRST_INSTANCE,
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

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let global_resources = GlobalResources::new(resource_man, &device, vertices, indices);
        let game_render_resources = GameRenderResources::new(&device, &config, &global_resources);

        surface.configure(&device, &config);

        AutomancyRenderResources {
            vsync,

            window,

            adapter_info: adapter.get_info(),
            instance,
            device,
            queue,
            surface,
            config,

            global_resources,
            game_render_resources,
        }
    }
}
