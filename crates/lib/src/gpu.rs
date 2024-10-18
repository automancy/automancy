use automancy_defs::rendering::{AnimationMatrixData, GameUBO, GpuInstance, MatrixData, Vertex};
use automancy_defs::rendering::{PostProcessingUBO, WorldMatrixData};
use automancy_defs::{rendering::IntermediateUBO, slice_group_by::GroupBy};
use automancy_macros::OptionGetter;
use automancy_resources::ResourceManager;
use bytemuck::Pod;
use ordermap::OrderMap;
use std::{collections::BTreeMap, mem};
use std::{num::NonZero, sync::Arc};
use wgpu::{
    util::{backend_bits_from_env, power_preference_from_env, BufferInitDescriptor, DeviceExt},
    BufferAddress, InstanceFlags, PipelineCompilationOptions, COPY_BUFFER_ALIGNMENT,
};
use wgpu::{
    util::{DrawIndexedIndirectArgs, StagingBelt},
    CommandEncoder,
};
use wgpu::{AdapterInfo, Face, Surface};
use wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferBindingType, BufferUsages, Color, ColorTargetState, ColorWrites, CompareFunction,
    DepthStencilState, Device, DeviceDescriptor, Extent3d, Features, FilterMode, FragmentState,
    FrontFace, Instance, InstanceDescriptor, Limits, MultisampleState, PipelineLayoutDescriptor,
    PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexState,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;
use yakui::UVec2;

pub const NORMAL_CLEAR: Color = Color::TRANSPARENT;
pub const MODEL_DEPTH_CLEAR: Color = Color {
    r: -1.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const MODEL_DEPTH_FORMAT: TextureFormat = TextureFormat::R32Float;
pub const SCREENSHOT_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

fn align_to_copy_alignment(add: BufferAddress) -> BufferAddress {
    add + (COPY_BUFFER_ALIGNMENT - (add % COPY_BUFFER_ALIGNMENT))
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

pub fn ordered_map_update_buffer<K, V>(queue: &Queue, buffer: &Buffer, data: &OrderMap<K, V>)
where
    V: Pod + Default,
{
    queue.write_buffer(buffer, 0, &ordered_map_write_to_buffer(data));
}

fn map_write_to_buffer<V>(data: &BTreeMap<u32, V>) -> Vec<u8>
where
    V: Pod + Default,
{
    let mut init_buffer = vec![];

    let max_index = data.keys().cloned().max().unwrap_or_default();

    for i in 0..=max_index {
        init_buffer.extend_from_slice(bytemuck::bytes_of(
            &data.get(&i).cloned().unwrap_or_default(),
        ));
    }

    init_buffer
}

pub fn map_update_buffer<V>(queue: &Queue, buffer: &Buffer, data: &BTreeMap<u32, V>)
where
    V: Pod + Default,
{
    queue.write_buffer(buffer, 0, &map_write_to_buffer(data));
}

#[must_use]
pub fn update_buffer_with_changes<V>(
    encoder: &mut CommandEncoder,
    device: &Device,
    buffer: &Buffer,
    changes: &[u32],
    data: &BTreeMap<u32, V>,
) -> Option<StagingBelt>
where
    V: Pod + Default,
{
    debug_assert!(changes.windows(2).all(|v| v[0] < v[1]));

    let byte_size = size_of::<V>();

    let entire_size = changes.len();
    if let Some(max_batch) = changes
        .linear_group_by(|a, b| b - a == 1)
        .map(|v| v.len())
        .max()
    {
        // TODO test a good multiplier for the size
        let mut belt = StagingBelt::new(align_to_copy_alignment(
            (byte_size * entire_size / 4).max(byte_size * max_batch) as BufferAddress,
        ));

        for batch in changes.linear_group_by(|a, b| b - a == 1) {
            if !batch.is_empty() {
                let mut view = belt.write_buffer(
                    encoder,
                    buffer,
                    (byte_size * batch[0] as usize) as BufferAddress,
                    unsafe { NonZero::new_unchecked((byte_size * batch.len()) as BufferAddress) },
                    device,
                );
                for (idx, v) in batch.iter().map(|i| data.get(i)).enumerate() {
                    match v {
                        Some(v) => view[(byte_size * idx)..(byte_size * (idx + 1))]
                            .copy_from_slice(bytemuck::bytes_of(v)),
                        None => view[(byte_size * idx)..(byte_size * (idx + 1))]
                            .copy_from_slice(bytemuck::bytes_of(&V::default())),
                    }
                }
            }
        }

        belt.finish();

        return Some(belt);
    }

    None
}

#[must_use]
pub fn resize_update_buffer_with_changes<V>(
    encoder: &mut CommandEncoder,
    device: &Device,
    buffer: &mut Buffer,
    changes: &[u32],
    data: &BTreeMap<u32, V>,
) -> Option<StagingBelt>
where
    V: Pod + Default,
{
    debug_assert!(changes.windows(2).all(|v| v[0] < v[1]));

    let byte_size = size_of::<V>();

    let max_index = changes.last().cloned().unwrap_or_default();
    let size = max_index as usize + 1;

    if (buffer.size() as usize) < byte_size * size {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &map_write_to_buffer(data),
            usage,
        });
    } else {
        return update_buffer_with_changes(encoder, device, buffer, changes, data);
    }

    None
}

pub fn resize_update_buffer<V>(device: &Device, queue: &Queue, buffer: &mut Buffer, data: &[V])
where
    V: Pod,
{
    if (buffer.size() as usize) < std::mem::size_of_val(data) {
        let usage = buffer.usage();

        *buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage,
        });
    } else {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }
}

pub fn clear_buffer(device: &Device, buffer: &mut Buffer) {
    let usage = buffer.usage();

    *buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: &[],
        usage,
    });
}

#[must_use]
pub fn update_indirect_buffer(
    encoder: &mut CommandEncoder,
    device: &Device,
    buffer: &mut Buffer,
    changes: &[u32],
    data: &BTreeMap<u32, DrawIndexedIndirectArgs>,
) -> Option<StagingBelt> {
    debug_assert!(changes.windows(2).all(|v| v[0] < v[1]));

    const BYTE_SIZE: usize = size_of::<DrawIndexedIndirectArgs>();

    let max_index = changes.last().cloned().unwrap_or_default();
    let size = max_index as usize + 1;

    if (buffer.size() as usize) < BYTE_SIZE * size {
        let usage = buffer.usage();

        let mut vec = Vec::<u8>::with_capacity(BYTE_SIZE * size);
        for i in 0..=max_index {
            vec.extend(data.get(&i).cloned().unwrap_or_default().as_bytes());
        }

        *buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &vec,
            usage,
        });
    } else {
        let entire_size = changes.len();
        if let Some(max_batch) = changes
            .linear_group_by(|a, b| b - a == 1)
            .map(|v| v.len())
            .max()
        {
            // TODO test a good multiplier for the size
            let mut belt = StagingBelt::new(align_to_copy_alignment(
                (BYTE_SIZE * entire_size / 4).max(BYTE_SIZE * max_batch) as BufferAddress,
            ));

            for batch in changes.linear_group_by(|a, b| b - a == 1) {
                if !batch.is_empty() {
                    let mut view = belt.write_buffer(
                        encoder,
                        buffer,
                        (BYTE_SIZE * batch[0] as usize) as BufferAddress,
                        unsafe {
                            NonZero::new_unchecked((BYTE_SIZE * batch.len()) as BufferAddress)
                        },
                        device,
                    );
                    for (idx, v) in batch.iter().map(|i| data.get(i)).enumerate() {
                        match v {
                            Some(v) => view[(BYTE_SIZE * idx)..(BYTE_SIZE * (idx + 1))]
                                .copy_from_slice(v.as_bytes()),
                            None => view[(BYTE_SIZE * idx)..(BYTE_SIZE * (idx + 1))]
                                .copy_from_slice(DrawIndexedIndirectArgs::default().as_bytes()),
                        }
                    }
                }
            }

            belt.finish();

            return Some(belt);
        }
    }

    None
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

pub fn make_fxaa_bind_group(
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

pub struct GameResources {
    pub instance_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub animation_matrix_data_buffer: Buffer,
    pub world_matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,
}

pub struct OverlayObjectsResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub world_matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,
}

#[derive(OptionGetter)]
pub struct GuiResources {
    pub instance_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub matrix_data_buffer: Buffer,
    pub animation_matrix_data_buffer: Buffer,
    pub world_matrix_data_buffer: Buffer,
    pub bind_group: BindGroup,

    #[getters(get)]
    pub color_texture: Option<Texture>,
    #[getters(get)]
    pub depth_texture: Option<Texture>,
    #[getters(get)]
    pub model_depth_texture: Option<Texture>,
    #[getters(get)]
    pub normal_texture: Option<Texture>,

    pub post_processing_uniform_buffer: Buffer,
    pub post_processing_bind_group_uniform: BindGroup,
    #[getters(get)]
    pub post_processing_bind_group_textures: Option<BindGroup>,

    #[getters(get)]
    pub post_processing_texture: Option<Texture>,
    #[getters(get)]
    pub antialiasing_bind_group: Option<BindGroup>,

    #[getters(get)]
    pub present_texture: Option<Texture>,
}

impl GuiResources {
    pub fn resize(
        &mut self,
        device: &Device,
        surface_format: TextureFormat,
        global_resources: &GlobalResources,
        size: UVec2,
    ) {
        self.color_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        self.depth_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        self.model_depth_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: MODEL_DEPTH_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        self.normal_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: NORMAL_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        let color = self
            .color_texture()
            .create_view(&TextureViewDescriptor::default());
        let normal = self
            .normal_texture()
            .create_view(&TextureViewDescriptor::default());
        let model_depth = self
            .model_depth_texture()
            .create_view(&TextureViewDescriptor::default());

        self.post_processing_bind_group_textures =
            Some(device.create_bind_group(&BindGroupDescriptor {
                layout: &global_resources.post_processing_bind_group_layout_textures,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Sampler(&global_resources.filtering_sampler),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&global_resources.nonfiltering_sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&global_resources.repeating_sampler),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&color),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&normal),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&model_depth),
                    },
                ],
                label: None,
            }));

        self.post_processing_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        self.antialiasing_bind_group = Some(make_fxaa_bind_group(
            device,
            &global_resources.fxaa_bind_group_layout,
            &self
                .post_processing_texture()
                .create_view(&TextureViewDescriptor::default()),
            &global_resources.filtering_sampler,
        ));

        self.present_texture = Some(device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: (size.x * 3) / 2,
                height: (size.y * 3) / 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));
    }
}

pub struct PostProcessingResources {
    pub bind_group_uniform: BindGroup,
    pub uniform_buffer: Buffer,
}

pub struct RenderResources {
    pub overlay_objects_resources: OverlayObjectsResources,
    pub game_resources: GameResources,

    pub gui_resources: Option<GuiResources>,

    pub post_processing_resources: PostProcessingResources,
}

pub struct GlobalResources {
    pub game_shader: ShaderModule,
    pub combine_shader: ShaderModule,
    pub intermediate_shader: ShaderModule,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,

    pub game_pipeline: RenderPipeline,

    pub intermediate_bind_group_layout: BindGroupLayout,
    pub screenshot_uniform_buffer: Buffer,
    pub screenshot_pipeline: RenderPipeline,
    pub present_uniform_buffer: Buffer,
    pub present_pipeline: RenderPipeline,
    pub multisampled_present_pipeline: RenderPipeline,

    pub post_processing_pipeline: RenderPipeline,
    pub post_processing_bind_group_layout_uniform: BindGroupLayout,
    pub post_processing_bind_group_layout_textures: BindGroupLayout,

    pub fxaa_pipeline: RenderPipeline,
    pub fxaa_bind_group_layout: BindGroupLayout,

    pub combine_pipeline: RenderPipeline,
    pub combine_bind_group_layout: BindGroupLayout,

    pub filtering_sampler: Sampler,
    pub nonfiltering_sampler: Sampler,
    pub repeating_sampler: Sampler,
}

#[derive(OptionGetter)]
pub struct SharedResources {
    #[getters(get)]
    game_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    gui_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    gui_texture_resolve: Option<(Texture, TextureView)>,
    #[getters(get)]
    normal_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    depth_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    model_depth_texture: Option<(Texture, TextureView)>,

    #[getters(get)]
    game_post_processing_bind_group: Option<BindGroup>,
    #[getters(get)]
    game_post_processing_texture: Option<(Texture, TextureView)>,
    #[getters(get)]
    game_antialiasing_bind_group: Option<BindGroup>,
    #[getters(get)]
    game_antialiasing_texture: Option<(Texture, TextureView)>,

    #[getters(get)]
    overlay_depth_texture: Option<(Texture, TextureView)>,

    #[getters(get)]
    first_combine_bind_group: Option<BindGroup>,
    #[getters(get)]
    first_combine_texture: Option<(Texture, TextureView)>,

    #[getters(get)]
    present_bind_group: Option<BindGroup>,
    #[getters(get)]
    screenshot_bind_group: Option<BindGroup>,
}

impl SharedResources {
    pub fn create(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
        global_resources: &GlobalResources,
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
        self.gui_texture = Some(create_texture_and_view(
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
        self.gui_texture_resolve = Some(create_texture_and_view(
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
                format: NORMAL_FORMAT,
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
        self.model_depth_texture = Some(create_texture_and_view(
            device,
            &TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: MODEL_DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        ));

        self.game_post_processing_bind_group =
            Some(device.create_bind_group(&BindGroupDescriptor {
                layout: &global_resources.post_processing_bind_group_layout_textures,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Sampler(&global_resources.filtering_sampler),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&global_resources.nonfiltering_sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&global_resources.repeating_sampler),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.game_texture().1),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&self.normal_texture().1),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&self.model_depth_texture().1),
                    },
                ],
                label: None,
            }));
        self.game_post_processing_texture = Some(create_texture_and_view(
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

pub fn init_gpu_resources(
    device: &Device,
    config: &SurfaceConfiguration,
    resource_man: &ResourceManager,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
) -> (SharedResources, RenderResources, GlobalResources) {
    let game_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Game Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["game"].to_string().into()),
    });

    let combine_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Combine Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["combine"].to_string().into()),
    });

    let fxaa_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("FXAA Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["fxaa"].to_string().into()),
    });

    let post_processing_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Post Processing Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["post_processing"].to_string().into()),
    });

    let intermediate_shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Intermediate Shader"),
        source: ShaderSource::Wgsl(resource_man.shaders["intermediate"].to_string().into()),
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

    let nonfiltering_sampler = device.create_sampler(&SamplerDescriptor {
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

    let post_processing_bind_group_layout_uniform =
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("post_processing_bind_group_layout_uniform"),
        });

    let post_processing_bind_group_layout_textures =
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
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
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
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
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
            label: Some("post_processing_bind_group_layout_textures"),
        });

    let game_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
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

    let game_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Game Render Pipeline Layout"),
        bind_group_layouts: &[&game_bind_group_layout],
        push_constant_ranges: &[],
    });

    let game_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Game Render Pipeline"),
        layout: Some(&game_pipeline_layout),
        vertex: VertexState {
            module: &game_shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc(), GpuInstance::desc()],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &game_shader,
            entry_point: "fs_main",
            targets: &[
                Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: NORMAL_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::COLOR,
                }),
                Some(ColorTargetState {
                    format: MODEL_DEPTH_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            ],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
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
        cache: None,
    });

    let game_resources = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Game Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Game Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<MatrixData>() * 524288],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let animation_matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Game Animation Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<AnimationMatrixData>() * 524288],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let world_matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Game World Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<WorldMatrixData>()],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &game_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: animation_matrix_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: world_matrix_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("game_bind_group"),
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
            animation_matrix_data_buffer,
            world_matrix_data_buffer,
            uniform_buffer,
            bind_group,
        }
    };

    let overlay_objects_resources = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Overlay Objects Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Overlay Objects Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<MatrixData>() * 256],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let world_matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Overlay Objects World Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<WorldMatrixData>() * 256],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &game_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: game_resources
                        .animation_matrix_data_buffer
                        .as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: world_matrix_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("overlay_objects_bind_group"),
        });

        OverlayObjectsResources {
            instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
            matrix_data_buffer,
            world_matrix_data_buffer,
            uniform_buffer,
            bind_group,
        }
    };

    let gui_resources = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GameUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        const MATRIX_DATA_SIZE: usize = 4096;
        let matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<MatrixData>() * MATRIX_DATA_SIZE],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let animation_matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui Animation Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<AnimationMatrixData>() * MATRIX_DATA_SIZE],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let world_matrix_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Gui World Matrix Data Buffer"),
            contents: &vec![0; mem::size_of::<WorldMatrixData>() * MATRIX_DATA_SIZE],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &game_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: animation_matrix_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: world_matrix_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("gui_bind_group"),
        });

        let post_processing_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[PostProcessingUBO {
                flags: 0,
                ..Default::default()
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let post_processing_bind_group_uniform = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &post_processing_bind_group_layout_uniform,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: post_processing_uniform_buffer.as_entire_binding(),
            }],
        });

        GuiResources {
            instance_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
            uniform_buffer,
            matrix_data_buffer,
            animation_matrix_data_buffer,
            world_matrix_data_buffer,
            bind_group,

            color_texture: None,
            depth_texture: None,
            model_depth_texture: None,
            normal_texture: None,

            post_processing_uniform_buffer,
            post_processing_bind_group_uniform,
            post_processing_bind_group_textures: None,

            post_processing_texture: None,
            antialiasing_bind_group: None,

            present_texture: None,
        }
    };

    let combine_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
    });

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
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &combine_shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
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
        cache: None,
    });

    let fxaa_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

    let fxaa_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Antialiasing Render Pipeline Layout"),
        bind_group_layouts: &[&fxaa_bind_group_layout],
        push_constant_ranges: &[],
    });

    let fxaa_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("FXAA Render Pipeline"),
        layout: Some(&fxaa_pipeline_layout),
        vertex: VertexState {
            module: &fxaa_shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &fxaa_shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
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
        cache: None,
    });

    let (post_processing_resources, post_processing_pipeline) = {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Post Processing Uniform Buffer"),
            contents: bytemuck::cast_slice(&[PostProcessingUBO::default()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_uniform = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &post_processing_bind_group_layout_uniform,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Post Processing Render Pipeline Layout"),
            bind_group_layouts: &[
                &post_processing_bind_group_layout_textures,
                &post_processing_bind_group_layout_uniform,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Post Processing Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &post_processing_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &post_processing_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
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
            cache: None,
        });

        (
            PostProcessingResources {
                uniform_buffer,
                bind_group_uniform,
            },
            pipeline,
        )
    };

    let intermediate_bind_group_layout =
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let intermediate_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Intermediate Render Pipeline Layout"),
        bind_group_layouts: &[&intermediate_bind_group_layout],
        push_constant_ranges: &[],
    });

    let screenshot_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Screenshot Uniform Buffer"),
        contents: bytemuck::cast_slice(&[IntermediateUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let screenshot_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Screenshot Render Pipeline"),
        layout: Some(&intermediate_pipeline_layout),
        vertex: VertexState {
            module: &intermediate_shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &intermediate_shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: SCREENSHOT_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
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
        cache: None,
    });

    let present_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Present Uniform Buffer"),
        contents: bytemuck::cast_slice(&[IntermediateUBO::default()]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let present_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Present Pipeline"),
        layout: Some(&intermediate_pipeline_layout),
        vertex: VertexState {
            module: &intermediate_shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &intermediate_shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
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
        cache: None,
    });

    let multisampled_present_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Present Pipeline"),
        layout: Some(&intermediate_pipeline_layout),
        vertex: VertexState {
            module: &intermediate_shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &intermediate_shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 4, // TODO this is a magic value!
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    let mut shared = SharedResources {
        game_texture: None,
        gui_texture: None,
        gui_texture_resolve: None,
        normal_texture: None,
        depth_texture: None,
        model_depth_texture: None,

        game_post_processing_bind_group: None,
        game_post_processing_texture: None,
        game_antialiasing_bind_group: None,
        game_antialiasing_texture: None,

        overlay_depth_texture: None,

        first_combine_bind_group: None,
        first_combine_texture: None,

        present_bind_group: None,
        screenshot_bind_group: None,
    };

    let render = RenderResources {
        overlay_objects_resources,
        game_resources,
        gui_resources: Some(gui_resources),
        post_processing_resources,
    };

    let global = GlobalResources {
        vertex_buffer,
        index_buffer,

        game_shader,
        combine_shader,
        intermediate_shader,

        game_pipeline,

        intermediate_bind_group_layout,
        screenshot_uniform_buffer,
        screenshot_pipeline,
        present_uniform_buffer,
        present_pipeline,
        multisampled_present_pipeline,

        post_processing_pipeline,
        post_processing_bind_group_layout_uniform,
        post_processing_bind_group_layout_textures,

        fxaa_pipeline,
        fxaa_bind_group_layout,

        combine_pipeline,
        combine_bind_group_layout,

        filtering_sampler,
        nonfiltering_sampler,
        repeating_sampler,
    };

    shared.create(device, config, &global);

    (shared, render, global)
}

pub struct Gpu {
    vsync: bool,

    pub window: Arc<Window>,

    pub adapter_info: AdapterInfo,
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
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

    pub fn resize(
        &mut self,
        shared_resources: &mut SharedResources,
        global_resources: &GlobalResources,
        size: PhysicalSize<u32>,
    ) {
        self.config.width = size.width;
        self.config.height = size.height;

        self.surface.configure(&self.device, &self.config);
        shared_resources.create(&self.device, &self.config, global_resources);
    }

    pub async fn new(window: Arc<Window>, vsync: bool) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor {
            backends: backend_bits_from_env().unwrap_or(Backends::all()),
            flags: InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: power_preference_from_env()
                    .unwrap_or(PowerPreference::HighPerformance),
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
                    memory_hints: Default::default(),
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
            vsync,

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
