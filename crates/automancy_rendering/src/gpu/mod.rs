use automancy_data::math::{UVec2, Vec2};
use automancy_game::{persistent::options::AAType, resources::ResourceManager};
use wgpu::util::DeviceExt;

use crate::{gpu, renderer};

pub mod data;
pub mod pipeline;
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

pub struct GlobalResources {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub mesh_data_buffer: wgpu::Buffer,

    pub compose_shader: wgpu::ShaderModule,
    pub fxaa_shader: wgpu::ShaderModule,
    pub game_lighting_shader: wgpu::ShaderModule,
    pub game_shader: wgpu::ShaderModule,
    pub post_processing_shader: wgpu::ShaderModule,
    pub blit_shader: wgpu::ShaderModule,

    pub point_sampler: wgpu::Sampler,
    pub bilinear_sampler: wgpu::Sampler,
    pub repeating_sampler: wgpu::Sampler,
}

impl GlobalResources {
    pub fn new(resource_man: &ResourceManager, render_state: &renderer::AutomancyRenderState, device: &wgpu::Device) -> Self {
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

        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(resource_man.shaders["blit"].to_string().into()),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&render_state.model_man.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&render_state.model_man.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // BTreeMap is sorted and meshes are ordered
        let mesh_data = render_state
            .model_man
            .meshes
            .values()
            .map(|mesh| gpu::data::GpuGameMeshData::new(mesh.transform))
            .collect::<Vec<_>>();

        let mesh_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Data Buffer"),
            contents: bytemuck::cast_slice(&mesh_data),
            usage: wgpu::BufferUsages::STORAGE,
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
            mesh_data_buffer,

            compose_shader,
            fxaa_shader,
            game_lighting_shader,
            game_shader,
            post_processing_shader,
            blit_shader,

            point_sampler,
            bilinear_sampler,
            repeating_sampler,
        }
    }
}

#[derive(Debug)]
pub struct GameRenderResources {
    pub render_textures: gpu::pipeline::GameRenderTextures,

    pub game_pipeline: gpu::pipeline::GamePipeline,
    pub game_lighting_pipeline: gpu::pipeline::GameLightingPipeline,
    pub post_processing_pipeline: gpu::pipeline::PostProcessingPipeline,
    pub fxaa_pipeline: Option<gpu::pipeline::FXAAPipeline>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GameRenderResources {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        antialiasing: AAType,
        old: Option<&Self>,
    ) -> Self {
        let render_textures = gpu::pipeline::GameRenderTextures::new(device, config, antialiasing);
        const DEFAULT_MODEL_MATRIX_SIZE: usize = std::mem::size_of::<gpu::data::GpuGameModelMatrixData>() * 128;
        const DEFAULT_WORLD_MATRIX_SIZE: usize = std::mem::size_of::<gpu::data::GpuGameWorldMatrixData>() * 8;
        const DEFAULT_ANIMATION_MATRIX_SIZE: usize = std::mem::size_of::<gpu::data::GpuGameAnimationMatrixData>() * 32;

        let game_pipeline = if let Some(old) = old {
            gpu::pipeline::GamePipeline::new(
                device,
                global_res,
                gpu::pipeline::GamePipelineArgs {
                    buffer: gpu::pipeline::GamePipelineBuffersArgs {
                        model_matrix_buffer: gpu::pipeline::BufferInitArg::Reuse(old.game_pipeline.model_matrix_buffer.clone()),
                        world_matrix_buffer: gpu::pipeline::BufferInitArg::Reuse(old.game_pipeline.world_matrix_buffer.clone()),
                        animation_matrix_buffer: gpu::pipeline::BufferInitArg::Reuse(old.game_pipeline.animation_matrix_buffer.clone()),
                        instance_buffer: gpu::pipeline::BufferInitArg::Reuse(old.game_pipeline.instance_buffer.clone()),
                        opaque_draw_buffer: Some(old.game_pipeline.opaque_draw_buffer.clone()),
                        non_opaque_draw_buffer: Some(old.game_pipeline.non_opaque_draw_buffer.clone()),
                    },
                },
            )
        } else {
            gpu::pipeline::GamePipeline::new(
                device,
                global_res,
                gpu::pipeline::GamePipelineArgs {
                    buffer: gpu::pipeline::GamePipelineBuffersArgs {
                        model_matrix_buffer: gpu::pipeline::BufferInitArg::init::<DEFAULT_MODEL_MATRIX_SIZE>(),
                        world_matrix_buffer: gpu::pipeline::BufferInitArg::init::<DEFAULT_WORLD_MATRIX_SIZE>(),
                        animation_matrix_buffer: gpu::pipeline::BufferInitArg::init::<DEFAULT_ANIMATION_MATRIX_SIZE>(),
                        ..Default::default()
                    },
                },
            )
        };

        let game_lighting_pipeline = gpu::pipeline::GameLightingPipeline::new(
            device,
            config,
            global_res,
            gpu::pipeline::GameLightingPipelineArgs {
                albedo_texture: &render_textures.albedo_texture,
                normal_texture: &render_textures.normal_texture,
                model_pos_texture: &render_textures.model_pos_texture,
            },
        );
        let post_processing_pipeline = gpu::pipeline::PostProcessingPipeline::new(
            device,
            config,
            global_res,
            gpu::pipeline::PostProcessingPipelineArgs {
                surface_texture: &render_textures.lighting_surface_texture,
                albedo_texture: &render_textures.albedo_texture,
                normal_texture: &render_textures.normal_texture,
                model_pos_texture: &render_textures.model_pos_texture,
            },
        );
        let fxaa_pipeline = match antialiasing {
            AAType::None => None,
            AAType::FXAA => Some(gpu::pipeline::FXAAPipeline::new(
                device,
                config,
                global_res,
                gpu::pipeline::FXAAPipelineArgs {
                    surface_texture: &render_textures.post_processing_surface_texture,
                },
            )),
        };

        Self {
            render_textures,

            game_pipeline,
            game_lighting_pipeline,
            post_processing_pipeline,
            fxaa_pipeline,
        }
    }

    #[inline]
    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        global_res: &gpu::GlobalResources,
        encoder: &mut wgpu::CommandEncoder,
        [opaque_draws, non_opaque_draws]: [u32; 2],
        (game_uniform, game_lighting_uniform, post_processing_uniform): (
            gpu::data::GpuGameUniformData,
            gpu::data::GpuGameLightingUniformData,
            gpu::data::GpuPostProcessingUniformData,
        ),
    ) {
        {
            #[cfg(feature = "profile")]
            profiling::scope!("game_render_pass");

            if self.game_pipeline.uniform_data != game_uniform {
                queue.write_buffer(&self.game_pipeline.uniform_buffer, 0, bytemuck::cast_slice(&[game_uniform]));
                self.game_pipeline.uniform_data = game_uniform;
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.render_textures.albedo_texture,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.render_textures.normal_texture,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(gpu::NORMAL_CLEAR),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.render_textures.model_pos_texture,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(gpu::MODEL_POS_CLEAR),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.render_textures.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.game_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.game_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &self.game_pipeline.bind_group_buffers, &[]);
            render_pass.set_vertex_buffer(0, global_res.vertex_buffer.slice(..));
            render_pass.set_index_buffer(global_res.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            if opaque_draws != 0 {
                render_pass.multi_draw_indexed_indirect(&self.game_pipeline.opaque_draw_buffer, 0, opaque_draws);
            }

            if non_opaque_draws != 0 {
                render_pass.multi_draw_indexed_indirect(&self.game_pipeline.non_opaque_draw_buffer, 0, non_opaque_draws);
            }
        }

        {
            #[cfg(feature = "profile")]
            profiling::scope!("game_lighting_render_pass");

            if self.game_lighting_pipeline.uniform_data != game_lighting_uniform {
                queue.write_buffer(
                    &self.game_lighting_pipeline.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[game_lighting_uniform]),
                );
                self.game_lighting_pipeline.uniform_data = game_lighting_uniform;
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Lighting Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_textures.lighting_surface_texture,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.game_lighting_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.game_lighting_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &self.game_lighting_pipeline.bind_group_samplers, &[]);
            render_pass.set_bind_group(2, &self.game_lighting_pipeline.bind_group_textures, &[]);
            render_pass.draw(0..3, 0..1);
        }

        {
            #[cfg(feature = "profile")]
            profiling::scope!("game_post_processing_render_pass");

            if self.post_processing_pipeline.uniform_data != post_processing_uniform {
                queue.write_buffer(
                    &self.post_processing_pipeline.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[post_processing_uniform]),
                );
                self.post_processing_pipeline.uniform_data = post_processing_uniform;
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Post Processing Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_textures.post_processing_surface_texture,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.post_processing_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.post_processing_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &self.post_processing_pipeline.bind_group_samplers, &[]);
            render_pass.set_bind_group(2, &self.post_processing_pipeline.bind_group_textures, &[]);
            render_pass.draw(0..3, 0..1);
        }

        if let Some(fxaa_pipeline) = &self.fxaa_pipeline {
            #[cfg(feature = "profile")]
            profiling::scope!("game_fxaa_render_pass");

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Game Antialiasing Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: self.render_textures.fxaa_surface_texture.as_ref().unwrap(),
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });

                render_pass.set_pipeline(&fxaa_pipeline.render_pipeline);
                render_pass.set_bind_group(0, &fxaa_pipeline.bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }
    }
}

pub struct GuiRenderResources {
    pub gui_texture_mxaa_view: wgpu::TextureView,
    pub gui_texture_mxaa: wgpu::Texture,
    pub gui_texture_view: wgpu::TextureView,
    pub gui_texture: wgpu::Texture,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GuiRenderResources {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, _global_res: &GlobalResources) -> Self {
        let gui_texture_mxaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Multisampled Gui Output Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            gui_texture_mxaa_view: gui_texture_mxaa.create_view(&wgpu::TextureViewDescriptor::default()),
            gui_texture_mxaa,
            gui_texture_view: gui_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            gui_texture,
        }
    }
}

pub struct PresentResources {
    pub compose_pipeline: gpu::pipeline::ComposePipeline,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl PresentResources {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        game_res: &GameRenderResources,
        overlay_res: &GameRenderResources,
        gui_res: &GuiRenderResources,
    ) -> Self {
        let compose_pipeline = gpu::pipeline::ComposePipeline::new(
            device,
            config,
            global_res,
            gpu::pipeline::ComposePipelineArgs {
                sampler: &global_res.point_sampler,
                textures: &[
                    &game_res.render_textures.output_texture,
                    &overlay_res.render_textures.output_texture,
                    &gui_res.gui_texture_view,
                ],
            },
        );

        Self {
            compose_pipeline,
        }
    }
}

pub struct RenderResources {
    pub adapter_info: wgpu::AdapterInfo,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    viewport_size_u32: UVec2,
    viewport_size_f32: Vec2,

    pub global_res: GlobalResources,

    pub main_game_res: GameRenderResources,
    pub overlay_game_res: GameRenderResources,
    pub gui_res: GuiRenderResources,
    pub present_res: PresentResources,

    vsync: bool,
    antialiasing: AAType,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl RenderResources {
    #[inline]
    fn pick_present_mode(vsync: bool) -> wgpu::PresentMode {
        if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        }
    }

    #[inline]
    pub fn is_vsync(&self) -> bool {
        self.vsync
    }

    #[inline]
    pub fn set_vsync(&mut self, vsync: bool) {
        if self.vsync != vsync {
            self.vsync = vsync;
            self.config.present_mode = Self::pick_present_mode(vsync);

            self.surface.configure(&self.device, &self.config);
        }
    }

    #[inline]
    pub fn set_antialiasing_type(&mut self, antialiasing: AAType) {
        if self.antialiasing != antialiasing {
            self.antialiasing = antialiasing;

            self.recreate();
        }
    }

    #[inline]
    pub fn get_antialiasing_type(&self) -> AAType {
        self.antialiasing
    }

    #[inline]
    pub fn viewport_size_u32(&self) -> UVec2 {
        self.viewport_size_u32
    }

    #[inline]
    pub fn viewport_size_f32(&self) -> Vec2 {
        self.viewport_size_f32
    }

    pub fn resize(&mut self, size: UVec2) {
        self.viewport_size_u32 = size;
        self.viewport_size_f32 = size.as_();

        self.config.width = self.viewport_size_u32.x;
        self.config.height = self.viewport_size_u32.y;

        self.recreate();
    }

    pub fn recreate(&mut self) {
        self.surface.configure(&self.device, &self.config);

        self.main_game_res = GameRenderResources::new(
            &self.device,
            &self.config,
            &self.global_res,
            self.antialiasing,
            Some(&self.main_game_res),
        );
        self.overlay_game_res = GameRenderResources::new(
            &self.device,
            &self.config,
            &self.global_res,
            self.antialiasing,
            Some(&self.overlay_game_res),
        );
        self.gui_res = GuiRenderResources::new(&self.device, &self.config, &self.global_res);

        self.present_res = PresentResources::new(
            &self.device,
            &self.config,
            &self.global_res,
            &self.main_game_res,
            &self.overlay_game_res,
            &self.gui_res,
        );
    }

    pub async fn new(
        resource_man: &ResourceManager,
        render_state: &renderer::AutomancyRenderState,
        window_handle: impl Into<wgpu::SurfaceTarget<'static>>,
        display_handle: impl wgpu::wgt::WgpuHasDisplayHandle,
    ) -> Self {
        let vsync = true;
        let antialiasing = AAType::None;

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display_handle)));

        let surface = instance.create_surface(window_handle).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::HighPerformance),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::INDIRECT_FIRST_INSTANCE
                    | wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                required_limits: wgpu::Limits {
                    max_binding_array_elements_per_shader_stage: 8192,
                    ..Default::default()
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let global_res = GlobalResources::new(resource_man, render_state, &device);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        log::info!("Surface format: {surface_format:?}");

        // dummy value
        let viewport_size_u32 = UVec2::one();
        let viewport_size_f32 = Vec2::one();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: viewport_size_u32.x,
            height: viewport_size_u32.y,
            present_mode: Self::pick_present_mode(vsync),
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        let main_game_res = GameRenderResources::new(&device, &config, &global_res, antialiasing, None);
        let overlay_game_res = GameRenderResources::new(&device, &config, &global_res, antialiasing, None);
        let gui_res = GuiRenderResources::new(&device, &config, &global_res);

        let present_res = PresentResources::new(&device, &config, &global_res, &main_game_res, &overlay_game_res, &gui_res);

        RenderResources {
            adapter_info: adapter.get_info(),
            instance,
            device,
            queue,
            surface,
            config,
            viewport_size_u32,
            viewport_size_f32,

            global_res,

            main_game_res,
            overlay_game_res,
            gui_res,
            present_res,

            vsync,
            antialiasing,
        }
    }
}
