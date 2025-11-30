use core::num::NonZeroU32;

use wgpu::util::DeviceExt;

use crate::gpu;

#[derive(Debug)]
pub struct ComposePipelineArgs<'a> {
    pub sampler: &'a wgpu::Sampler,
    pub textures: &'a [&'a wgpu::TextureView],
}

#[derive(Debug)]
pub struct ComposePipeline {
    pub render_pipeline: wgpu::RenderPipeline,

    pub textures_bind_group: wgpu::BindGroup,
    pub uniform_bind_group: wgpu::BindGroup,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ComposePipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &gpu::GlobalResources,
        ComposePipelineArgs {
            sampler,
            textures,
        }: ComposePipelineArgs,
    ) -> Self {
        let textures_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compose_textures_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
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
                    count: Some(NonZeroU32::new(textures.len() as u32).unwrap()),
                },
            ],
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compose_uniform_bind_group_layout"),
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

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compose Render Pipeline Layout"),
            bind_group_layouts: &[Some(&textures_bind_group_layout), Some(&uniform_bind_group_layout)],
            immediate_size: 0,
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
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compose_textures_bind_group"),
            layout: &textures_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(textures),
                },
            ],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compose_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&[textures.len() as u32]),
                        usage: wgpu::BufferUsages::UNIFORM,
                    }),
                    offset: 0,
                    size: None,
                }),
            }],
        });

        ComposePipeline {
            render_pipeline,
            textures_bind_group,
            uniform_bind_group,
        }
    }
}
