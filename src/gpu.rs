use core::slice;
use std::mem;
use std::rc::Rc;

use egui_wgpu::wgpu::util::{BufferInitDescriptor, DeviceExt};
use egui_wgpu::wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferUsages, Color, ColorTargetState, ColorWrites,
    CompareFunction, DepthStencilState, Device, DeviceDescriptor, Extent3d, Features, FilterMode,
    FragmentState, FrontFace, Instance, InstanceDescriptor, Limits, MultisampleState,
    PipelineLayoutDescriptor, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    Queue, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexState,
};
use hashbrown::HashMap;
use image::EncodableLayout;
use wgpu::util::{DrawIndexedIndirectArgs, TextureDataOrder};
use wgpu::{AdapterInfo, Surface};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use automancy_defs::bytemuck;
use automancy_defs::id::Id;
use automancy_defs::math::Matrix4;
use automancy_defs::rendering::{GameUBO, InstanceData, MatrixData, RawInstanceData, Vertex};
use automancy_defs::slice_group_by::GroupBy;
use automancy_macros::OptionGetter;
use automancy_resources::ResourceManager;

use crate::SSAO_NOISE_MAP;

pub const GPU_BACKENDS: Backends = Backends::all();

pub const NORMAL_CLEAR: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 0.0,
};

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const SCREENSHOT_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub type AnimationMap = HashMap<Id, HashMap<usize, Matrix4>>;

pub fn init_gpu_resources(
    device: &Device,
    queue: &Queue,
    config: &SurfaceConfiguration,
    resource_man: &ResourceManager,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
) -> (
    SharedResources,
    RenderResources,
    GlobalBuffers,
    GuiResources,
) {
    let game_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Game Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["game"].as_str().into()),
    });

    let combine_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Combine Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["combine"].as_str().into()),
    });

    let fxaa_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("FXAA Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["fxaa"].as_str().into()),
    });

    let post_processing_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Post Processing Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["post_processing"].as_str().into()),
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

    let ssao_noise_map = image::load_from_memory(SSAO_NOISE_MAP)
        .unwrap()
        .to_rgba32f();
    let ssao_noise_map = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label: None,
            size: Extent3d {
                width: ssao_noise_map.width(),
                height: ssao_noise_map.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::LayerMajor,
        ssao_noise_map.as_bytes(),
    );

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

    let repeating_sampler = device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
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

        const MATRIX_DATA_SIZE: usize = 65536;
        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Game Matrix Data Buffer"),
            contents: &Vec::from_iter(
                (0..(mem::size_of::<MatrixData>() * MATRIX_DATA_SIZE)).map(|_| 0),
            ),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("game_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
            ],
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
                depth_compare: CompareFunction::Less,
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
            matrix_data_buffer,
            uniform_buffer,
            bind_group,
            pipeline,
            post_processing_bind_group: None,
            post_processing_texture: None,
            antialiasing_bind_group: None,
            antialiasing_texture: None,
        }
    };

    let in_world_item_resources = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("In-world Item Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        const MATRIX_DATA_SIZE: usize = 2048;
        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("In-world Item Matrix Data Buffer"),
            contents: &Vec::from_iter(
                (0..(mem::size_of::<MatrixData>() * MATRIX_DATA_SIZE)).map(|_| 0),
            ),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("in_world_item_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("in_world_item_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("In-World Item Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("In-world Item Render Pipeline"),
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
                depth_compare: CompareFunction::Less,
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
            matrix_data_buffer,
            bind_group,
            pipeline,
        }
    };

    let gui_resources = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        const MATRIX_DATA_SIZE: usize = 256;
        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui Matrix Data Buffer"),
            contents: &Vec::from_iter(
                (0..(mem::size_of::<MatrixData>() * MATRIX_DATA_SIZE)).map(|_| 0),
            ),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("gui_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
            ],
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
                module: &game_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), RawInstanceData::desc()],
            },
            fragment: Some(FragmentState {
                module: &game_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
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
                depth_compare: CompareFunction::Less,
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

        GuiResources {
            instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
            uniform_buffer,
            matrix_data_buffer,
            bind_group,
            pipeline,
        }
    };

    let egui_resources = EguiResources {
        texture: None,
        depth_texture: None,
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

        let fxaa_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("FXAA Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &fxaa_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &fxaa_shader,
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
            fxaa_pipeline,
        }
    };

    let post_processing_resources = {
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
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 9,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
            label: Some("post_processing_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Post Processing Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Post Processing Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &post_processing_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &post_processing_shader,
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

        PostProcessingResources {
            bind_group_layout,
            pipeline,
            ssao_noise_map,
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

    let mut shared = SharedResources {
        game_shader,
        combine_shader,
        intermediate_shader,

        game_texture: None,
        normal_texture: None,
        depth_texture: None,
        model_texture: None,
        multisampling_texture: None,
        multisampling_depth_texture: None,

        filtering_sampler,
        non_filtering_sampler,
        repeating_sampler,
    };

    let mut render = RenderResources {
        game_resources,
        in_world_item_resources,
        egui_resources,
        first_combine_resources,
        antialiasing_resources,
        post_processing_resources,
        intermediate_resources,
    };

    shared.create(device, config, &mut render);

    (
        shared,
        render,
        GlobalBuffers {
            vertex_buffer,
            index_buffer,
        },
        gui_resources,
    )
}

pub fn compile_instances<T: Clone + Send>(
    resource_man: &ResourceManager,
    instances: &[(InstanceData, Id, T)],
    animation_map: &AnimationMap,
) -> (
    HashMap<Id, Vec<(usize, RawInstanceData, T)>>,
    Vec<MatrixData>,
) {
    let mut raw_instances = HashMap::new();
    let mut matrix_data = vec![];

    #[cfg(debug_assertions)]
    let mut seen = hashbrown::HashSet::new();

    instances.binary_group_by_key(|v| v.1).for_each(|group| {
        let id = group[0].1;

        #[cfg(debug_assertions)]
        {
            if seen.contains(&id) {
                panic!("Duplicate id when collecting instances - are the instances sorted?");
            }
            seen.insert(id);
        }

        let models = &resource_man.all_models[&id].0;

        let vec = raw_instances
            .entry(id)
            .or_insert_with(|| Vec::with_capacity(group.len()));

        for (instance, _, extra) in group.iter() {
            for model in models.values() {
                let mut instance = *instance;

                let mut matrix = model.matrix;
                if let Some(anim) = animation_map
                    .get(&id)
                    .and_then(|anim| anim.get(&model.index))
                {
                    matrix *= *anim;
                }
                instance = instance.add_model_matrix(matrix);

                vec.push((
                    model.index,
                    RawInstanceData::from_instance(instance, &mut matrix_data),
                    extra.clone(),
                ));
            }
        }
    });

    raw_instances
        .values_mut()
        .for_each(|v| v.sort_by_key(|v| v.0));

    (raw_instances, matrix_data)
}

pub fn indirect_instance<T: Clone + Send + Sync>(
    resource_man: &ResourceManager,
    instances: &[(InstanceData, Id, T)],
    group: bool,
    animation_map: &AnimationMap,
) -> (
    Vec<RawInstanceData>,
    HashMap<Id, Vec<(DrawIndexedIndirectArgs, T)>>,
    u32,
    Vec<MatrixData>,
) {
    let (compiled_instances, matrix_data) =
        compile_instances(resource_man, instances, animation_map);

    let (_, draw_count, commands) = compiled_instances.iter().fold(
        (0, 0, HashMap::new()),
        |(mut base_instance_counter, mut draw_count, mut commands), (id, instances)| {
            let mut a = instances.binary_group_by_key(|v| v.0);
            let mut b = instances.iter().map(slice::from_ref);

            let vec = commands
                .entry(*id)
                .or_insert_with(|| Vec::with_capacity(instances.len()));

            for instances in if group {
                &mut a as &mut dyn Iterator<Item = &[(usize, RawInstanceData, T)]>
            } else {
                &mut b as &mut dyn Iterator<Item = &[(usize, RawInstanceData, T)]>
            } {
                let size = instances.len() as u32;

                let index_range = resource_man.all_index_ranges[id][&instances[0].0];

                let command = DrawIndexedIndirectArgs {
                    first_index: index_range.offset,
                    index_count: index_range.size,
                    first_instance: base_instance_counter,
                    instance_count: size,
                    base_vertex: 0,
                };

                base_instance_counter += size;
                draw_count += 1;

                vec.push((command, instances[0].2.clone()));
            }

            (base_instance_counter, draw_count, commands)
        },
    );

    let instances = compiled_instances
        .into_iter()
        .flat_map(|v| v.1.into_iter().map(|v| v.1))
        .collect::<Vec<RawInstanceData>>();

    (instances, commands, draw_count, matrix_data)
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
    frame_texture: &TextureView,
    frame_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(frame_texture),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(frame_sampler),
            },
        ],
        label: Some("antialiasing_bind_group"),
    })
}

fn make_post_processing_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    frame_texture: &TextureView,
    frame_sampler: &Sampler,
    depth_texture: &TextureView,
    depth_sampler: &Sampler,
    normal_texture: &TextureView,
    normal_sampler: &Sampler,
    model_texture: &TextureView,
    model_sampler: &Sampler,
    noise_texture: &TextureView,
    noise_sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(frame_texture),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(frame_sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(depth_texture),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(depth_sampler),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(normal_texture),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::Sampler(normal_sampler),
            },
            BindGroupEntry {
                binding: 6,
                resource: BindingResource::TextureView(model_texture),
            },
            BindGroupEntry {
                binding: 7,
                resource: BindingResource::Sampler(model_sampler),
            },
            BindGroupEntry {
                binding: 8,
                resource: BindingResource::TextureView(noise_texture),
            },
            BindGroupEntry {
                binding: 9,
                resource: BindingResource::Sampler(noise_sampler),
            },
        ],
        label: Some("post_processing_bind_group"),
    })
}

pub struct SharedDescriptor<'a> {
    pub filtering_sampler: &'a Sampler,
    pub non_filtering_sampler: &'a Sampler,
    pub repeating_sampler: &'a Sampler,

    pub game_texture: &'a TextureView,
    pub normal_texture: &'a TextureView,
    pub depth_texture: &'a TextureView,
    pub model_texture: &'a TextureView,
}

pub struct GameDescriptor<'a> {
    pub antialiasing_bind_group_layout: &'a BindGroupLayout,
    pub post_processing_bind_group_layout: &'a BindGroupLayout,
    pub ssao_noise_map: &'a TextureView,
}

#[derive(OptionGetter)]
pub struct GameResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
    #[getters(get)]
    post_processing_bind_group: Option<BindGroup>,
    #[getters(get)]
    post_processing_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    antialiasing_bind_group: Option<BindGroup>,
    #[getters(get)]
    antialiasing_texture: Option<(Texture, TextureView)>,
}

impl GameResources {
    pub fn create(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
        shared_descriptor: &SharedDescriptor,
        game_descriptor: &GameDescriptor,
    ) {
        self.post_processing_bind_group = Some(make_post_processing_bind_group(
            device,
            game_descriptor.post_processing_bind_group_layout,
            shared_descriptor.game_texture,
            shared_descriptor.filtering_sampler,
            shared_descriptor.depth_texture,
            shared_descriptor.non_filtering_sampler,
            shared_descriptor.normal_texture,
            shared_descriptor.non_filtering_sampler,
            shared_descriptor.model_texture,
            shared_descriptor.non_filtering_sampler,
            game_descriptor.ssao_noise_map,
            shared_descriptor.repeating_sampler,
        ));
        self.post_processing_texture = Some(create_texture_and_view(
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
        self.antialiasing_bind_group = Some(make_antialiasing_bind_group(
            device,
            game_descriptor.antialiasing_bind_group_layout,
            &self.post_processing_texture().1,
            shared_descriptor.filtering_sampler,
        ));
        self.antialiasing_texture = Some(create_texture_and_view(
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
    }
}

#[derive(OptionGetter)]
pub struct InWorldItemResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
}

#[derive(OptionGetter)]
pub struct GuiResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: RenderPipeline,
}

#[derive(OptionGetter)]
pub struct EguiResources {
    #[getters(get)]
    texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    depth_texture: Option<(Texture, TextureView)>,
}

impl EguiResources {
    pub fn create(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.texture = Some(create_texture_and_view(
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
        self.depth_texture = Some(create_texture_and_view(
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
                format: DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
    }
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

impl CombineResources {
    pub fn create(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
        shared_descriptor: &SharedDescriptor,
        first_texture: &TextureView,
        second_texture: &TextureView,
    ) {
        self.texture = Some(create_texture_and_view(
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
        self.bind_group = Some(make_combine_bind_group(
            device,
            &self.bind_group_layout,
            first_texture,
            shared_descriptor.filtering_sampler,
            second_texture,
            shared_descriptor.filtering_sampler,
        ));
    }
}

#[derive(OptionGetter)]
pub struct AntialiasingResources {
    pub bind_group_layout: BindGroupLayout,
    pub fxaa_pipeline: RenderPipeline,
}

#[derive(OptionGetter)]
pub struct PostProcessingResources {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
    pub ssao_noise_map: Texture,
}

#[derive(OptionGetter)]
pub struct IntermediateResources {
    pub bind_group_layout: BindGroupLayout,
    pub screenshot_pipeline: RenderPipeline,
    pub present_pipeline: RenderPipeline,
    #[getters(get)]
    present_bind_group: Option<BindGroup>,
}

impl IntermediateResources {
    pub fn create(
        &mut self,
        device: &Device,
        shared_descriptor: &SharedDescriptor,
        present_texture: &TextureView,
    ) {
        self.present_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(present_texture),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(shared_descriptor.non_filtering_sampler),
                },
            ],
        }));
    }
}

pub struct GlobalBuffers {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

#[derive(OptionGetter)]
pub struct SharedResources {
    pub game_shader: ShaderModule,
    pub combine_shader: ShaderModule,
    pub intermediate_shader: ShaderModule,

    #[getters(get)]
    game_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    normal_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    depth_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    model_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    multisampling_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    multisampling_depth_texture: Option<(Texture, TextureView)>,

    pub filtering_sampler: Sampler,
    pub non_filtering_sampler: Sampler,
    pub repeating_sampler: Sampler,
}

pub struct RenderResources {
    pub game_resources: GameResources,
    pub in_world_item_resources: InWorldItemResources,
    pub egui_resources: EguiResources,

    pub first_combine_resources: CombineResources,

    pub antialiasing_resources: AntialiasingResources,
    pub post_processing_resources: PostProcessingResources,
    pub intermediate_resources: IntermediateResources,
}

impl SharedResources {
    pub fn create(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
        render_resources: &mut RenderResources,
    ) {
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
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.model_texture = Some(create_texture_and_view(
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
        self.multisampling_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 4,
                dimension: TextureDimension::D2,
                format: config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));
        self.multisampling_depth_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 4,
                dimension: TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        let shared_descriptor = SharedDescriptor {
            filtering_sampler: &self.filtering_sampler,
            non_filtering_sampler: &self.non_filtering_sampler,
            repeating_sampler: &self.repeating_sampler,
            game_texture: &self.game_texture().1,
            normal_texture: &self.normal_texture().1,
            depth_texture: &self.depth_texture().1,
            model_texture: &self.model_texture().1,
        };

        let game_descriptor = GameDescriptor {
            antialiasing_bind_group_layout: &render_resources
                .antialiasing_resources
                .bind_group_layout,
            post_processing_bind_group_layout: &render_resources
                .post_processing_resources
                .bind_group_layout,
            ssao_noise_map: &render_resources
                .post_processing_resources
                .ssao_noise_map
                .create_view(&TextureViewDescriptor::default()),
        };

        render_resources.game_resources.create(
            device,
            config,
            &shared_descriptor,
            &game_descriptor,
        );
        render_resources.egui_resources.create(device, config);

        render_resources.first_combine_resources.create(
            device,
            config,
            &shared_descriptor,
            &render_resources.game_resources.antialiasing_texture().1,
            &render_resources.egui_resources.texture().1,
        );
        render_resources.intermediate_resources.create(
            device,
            &shared_descriptor,
            &render_resources.first_combine_resources.texture().1,
        );
    }
}

pub struct Gpu<'a> {
    vsync: bool,

    pub window: &'a Window,

    pub adapter_info: AdapterInfo,
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'a>,
    pub config: SurfaceConfiguration,
}

impl<'a> Gpu<'a> {
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

    pub fn resize(
        &mut self,
        shared_resources: &mut SharedResources,
        render_resources: &mut RenderResources,
        size: PhysicalSize<u32>,
    ) {
        self.config.width = size.width;
        self.config.height = size.height;

        self.surface.configure(&self.device, &self.config);
        shared_resources.create(&self.device, &self.config, render_resources);
    }

    pub async fn new(window: &'a Window, vsync: bool) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor {
            backends: GPU_BACKENDS,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

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
                    required_features: Features::INDIRECT_FIRST_INSTANCE
                        | Features::MULTI_DRAW_INDIRECT,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
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

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Gpu {
            vsync: false,

            window,

            adapter_info: adapter.get_info(),
            instance,
            device,
            queue,
            surface,
            config,
        }
    }
}
