use core::ops::Mul;
use std::{borrow::Cow, collections::VecDeque, time::Instant};

use automancy_data::{
    game::coord::TileCoord,
    id::{Id, ModelId, RenderId},
    rendering::{
        colors::Rgba,
        gpu::{GameDrawInstance, GameUniformData, NO_COLOR_OFFSET},
    },
};
use automancy_game::{
    actor::message::GameMsg, scripting::render::RenderCommand, state::AutomancyGameState,
};
use hashbrown::HashMap;
use yakui::{Rect, UVec2};

use crate::{
    data::{GpuGameLightingUniformData, GpuGameUniformData, GpuPostProcessingUniformData},
    gpu,
    gpu::{RenderResources, SCREENSHOT_FORMAT},
    renderer::AutomancyRenderState,
};

/*
pub struct YakuiRenderResources {
    pub instances: Option<Vec<GuiInstance>>,

    pub resource_man: Arc<ResourceManager>,
    pub global_resources: Arc<GlobalResources>,
    pub surface_format: TextureFormat,
    pub gui_resources: Option<GuiResources>,

    pub animation_matrix_data_map: Option<AnimationMatrixDataMap>,
    pub opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,
    pub non_opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,

    pub packed_size: Option<UVec2>,
    pub rects: Vec<Option<crunch::Rect>>,
}
*/

pub struct GameRenderer {
    tile_draw_ids: HashMap<TileCoord, Vec<(RenderId, ModelId)>>,

    pub tile_tints: HashMap<TileCoord, Rgba>,
    last_tile_tints: HashMap<TileCoord, Rgba>,

    pub take_item_animations: HashMap<Id, VecDeque<(Instant, Rect)>>,

    gui_packed_size: Option<UVec2>,
    gui_rects: Vec<Option<crunch::Rect>>,

    screenshot_clipboard: arboard::Clipboard,
}

impl GameRenderer {
    pub fn new() -> Self {
        Self {
            tile_draw_ids: Default::default(),

            tile_tints: Default::default(),
            last_tile_tints: Default::default(),

            take_item_animations: Default::default(),

            gui_packed_size: Default::default(),
            gui_rects: Default::default(),

            screenshot_clipboard: arboard::Clipboard::new().unwrap(),
        }
    }

    pub fn render(
        &mut self,
        game_state: &AutomancyGameState,
        render_state: &mut AutomancyRenderState,
        res: &mut RenderResources,
        screenshotting: bool,
    ) -> Result<(), wgpu::SurfaceError> {
        let size = res.window.inner_size();

        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let last_tile_tints = std::mem::take(&mut self.last_tile_tints);
        let tile_tints = std::mem::take(&mut self.tile_tints);

        let render_commands = {
            let culling_range = game_state.camera.culling_range;
            let game = game_state.game.clone();

            game_state
                .tokio
                .block_on(game.call(
                    |reply| GameMsg::GetAllRenderCommands {
                        reply,
                        culling_range,
                    },
                    None,
                ))
                .unwrap()
                .unwrap()
        };

        let world_matrix = game_state.camera.get_matrix();
        render_state.instance_man.set_all_world_matrix(world_matrix);

        for batch in render_commands {
            for (coord, commands) in batch {
                for command in commands {
                    match command {
                        RenderCommand::Track {
                            render_id,
                            model_id,
                        } => {
                            self.tile_draw_ids
                                .entry(coord)
                                .or_default()
                                .push((render_id, model_id));

                            render_state.instance_man.insert(
                                &render_state.model_man,
                                render_id,
                                model_id,
                                GameDrawInstance {
                                    world_matrix,
                                    ..Default::default()
                                },
                            );
                        }
                        RenderCommand::Untrack {
                            render_id,
                            model_id,
                        } => {
                            render_state.instance_man.remove(render_id, model_id);
                        }
                        RenderCommand::Transform {
                            render_id,
                            model_id,
                            model_matrix,
                        } => {
                            render_state.instance_man.set_matrix(
                                &render_state.model_man,
                                render_id,
                                model_id,
                                Some(model_matrix),
                                None,
                            );
                        }
                    }
                }
            }
        }

        {
            for (coord, _) in &last_tile_tints {
                if tile_tints.contains_key(coord) {
                    continue;
                };

                let Some(ids) = self.tile_draw_ids.get(coord) else {
                    continue;
                };

                for (render_id, model_id) in ids {
                    render_state.instance_man.inspect_instances(
                        &render_state.model_man,
                        *render_id,
                        *model_id,
                        |_, instance| {
                            instance.color_offset = NO_COLOR_OFFSET.into_array();
                        },
                    );
                }
            }

            for (coord, tint) in &tile_tints {
                let Some(ids) = self.tile_draw_ids.get(coord) else {
                    continue;
                };

                for (render_id, model_id) in ids {
                    render_state.instance_man.inspect_instances(
                        &render_state.model_man,
                        *render_id,
                        *model_id,
                        |_, instance| {
                            instance.color_offset = tint.into_array();
                        },
                    );
                }
            }
        }

        self.last_tile_tints = tile_tints;

        render_state.instance_man.upload_animation(
            game_state.start_instant,
            &render_state.model_man,
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.animation_matrix_data_buffer,
        );

        let [opaque_draws, non_opaque_draws] = render_state.instance_man.collect_draw_calls(
            &render_state.model_man,
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.model_matrix_data_buffer,
            &mut res.main_game_res.game_pipeline.world_matrix_data_buffer,
            &mut res.main_game_res.game_pipeline.opaque_instance_buffer,
            &mut res.main_game_res.game_pipeline.non_opaque_instance_buffer,
        );

        gpu::util::upload_buffer(
            &res.device,
            &res.queue,
            &mut res
                .main_game_res
                .game_pipeline
                .opaque_indirect_draw_command_buffer,
            &opaque_draws,
        );
        gpu::util::upload_buffer(
            &res.device,
            &res.queue,
            &mut res
                .main_game_res
                .game_pipeline
                .non_opaque_indirect_draw_command_buffer,
            &non_opaque_draws,
        );

        res.queue.submit([]);

        self.inner_render(game_state, res, screenshotting)?;

        Ok(())
    }
}

impl Default for GameRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRenderer {
    fn inner_render(
        &mut self,
        game_state: &AutomancyGameState,
        res: &mut RenderResources,
        screenshotting: bool,
    ) -> Result<(), wgpu::SurfaceError> {
        let size = res.window.inner_size();
        let surface = res.surface.get_current_texture()?;

        {
            let output_size = surface.texture.size();

            if output_size.width != size.width || output_size.height != size.height {
                return Ok(());
            }
        }

        let surface_pixel_size = surface.texture.format().target_pixel_byte_cost().unwrap();
        let surface_dim = surface.texture.size();
        let surface_buffer_dim = {
            let dim = surface_dim.physical_size(surface.texture.format());

            let padded_width = dim
                .width
                .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                .mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

            wgpu::Extent3d {
                width: padded_width,
                ..dim
            }
        };
        let surface_buffer_size =
            ((surface_buffer_dim.width * surface_buffer_dim.height * surface_pixel_size) as u64)
                .div_ceil(wgpu::COPY_BUFFER_ALIGNMENT)
                .mul(wgpu::COPY_BUFFER_ALIGNMENT);

        let view_desc = wgpu::TextureViewDescriptor::default();

        let mut encoder = res
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let game_uniform = GameUniformData {
            camera_pos: game_state.camera.get_pos(),
            ..Default::default()
        };

        {
            res.queue.write_buffer(
                &res.main_game_res.game_pipeline.uniform_buffer,
                0,
                bytemuck::cast_slice(&[GpuGameUniformData::new(&game_uniform)]),
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &res
                            .main_game_res
                            .render_textures
                            .albedo_texture
                            .create_view(&view_desc),
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &res
                            .main_game_res
                            .render_textures
                            .normal_texture
                            .create_view(&view_desc),
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(gpu::NORMAL_CLEAR),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &res
                            .main_game_res
                            .render_textures
                            .model_pos_texture
                            .create_view(&view_desc),
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(gpu::MODEL_POS_CLEAR),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &res
                        .main_game_res
                        .render_textures
                        .depth_texture
                        .create_view(&view_desc),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&res.main_game_res.game_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &res.main_game_res.game_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &res.main_game_res.game_pipeline.bind_group_buffers, &[]);
            render_pass.set_vertex_buffer(0, res.global_res.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                res.global_res.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.set_vertex_buffer(
                1,
                res.main_game_res
                    .game_pipeline
                    .opaque_instance_buffer
                    .slice(..),
            );
            render_pass.draw_indexed_indirect(
                &res.main_game_res
                    .game_pipeline
                    .opaque_indirect_draw_command_buffer,
                0,
            );

            render_pass.set_vertex_buffer(
                1,
                res.main_game_res
                    .game_pipeline
                    .non_opaque_instance_buffer
                    .slice(..),
            );
            render_pass.draw_indexed_indirect(
                &res.main_game_res
                    .game_pipeline
                    .non_opaque_indirect_draw_command_buffer,
                0,
            );
        }

        {
            res.queue.write_buffer(
                &res.main_game_res.game_lighting_pipeline.uniform_buffer,
                0,
                bytemuck::cast_slice(&[GpuGameLightingUniformData::new(&game_uniform)]),
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Lighting Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res
                        .main_game_res
                        .render_textures
                        .lighting_surface_texture
                        .create_view(&view_desc),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&res.main_game_res.game_lighting_pipeline.render_pipeline);
            render_pass.set_bind_group(
                0,
                &res.main_game_res.game_lighting_pipeline.bind_group_uniform,
                &[],
            );
            render_pass.set_bind_group(
                1,
                &res.main_game_res.game_lighting_pipeline.bind_group_samplers,
                &[],
            );
            render_pass.set_bind_group(
                2,
                &res.main_game_res.game_lighting_pipeline.bind_group_textures,
                &[],
            );
            render_pass.draw(0..3, 0..1);
        }

        {
            res.queue.write_buffer(
                &res.main_game_res.post_processing_pipeline.uniform_buffer,
                0,
                bytemuck::cast_slice(&[GpuPostProcessingUniformData {
                    ..Default::default()
                }]),
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Post Processing Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res
                        .main_game_res
                        .render_textures
                        .post_processing_surface_texture
                        .create_view(&view_desc),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&res.main_game_res.post_processing_pipeline.render_pipeline);
            render_pass.set_bind_group(
                0,
                &res.main_game_res
                    .post_processing_pipeline
                    .bind_group_uniform,
                &[],
            );
            render_pass.set_bind_group(
                1,
                &res.main_game_res
                    .post_processing_pipeline
                    .bind_group_samplers,
                &[],
            );
            render_pass.set_bind_group(
                2,
                &res.main_game_res
                    .post_processing_pipeline
                    .bind_group_textures,
                &[],
            );
            render_pass.draw(0..3, 0..1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Antialiasing Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res
                        .main_game_res
                        .render_textures
                        .fxaa_surface_texture
                        .create_view(&view_desc),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&res.main_game_res.fxaa_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &res.main_game_res.fxaa_pipeline.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res.gui_res.gui_texture.create_view(&view_desc),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }
        /*
               let custom_gui_commands: CommandBuffer;
               {
                   let surface_info = SurfaceInfo {
                       format: self.gpu.config.format,
                       sample_count: 4,
                       color_attachment: Some(RenderPassColorAttachment {
                           view: &self.shared_resources.gui_texture().1,
                           depth_slice: None,
                           resolve_target: Some(&self.shared_resources.gui_texture_resolve().1),
                           ops: Operations {
                               load: LoadOp::Clear(Color::TRANSPARENT),
                               store: StoreOp::Store,
                           },
                       }),
                   };

                   let mut resources = YakuiRenderResources {
                       instances: Some(Vec::new()),

                       resource_man: resource_man.clone(),
                       global_resources: self.global_resources.clone(),
                       surface_format: surface_info.format,
                       gui_resources: self.render_resources.gui_resources.take(),

                       animation_matrix_data_map: self.gui_animation_matrix_data_map.take(),
                       opaque_draws: self.gui_opaque_draws.take(),
                       non_opaque_draws: self.gui_non_opaque_draws.take(),

                       packed_size: self.gui_packed_size.take(),
                       rects: std::mem::take(&mut self.gui_rects),

                       animation_cache: std::mem::take(&mut self.animation_cache),
                   };

                   {
                       let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                           label: Some("yakui Render Pass"),
                           color_attachments: &surface_info.color_attachments,
                           depth_stencil_attachment: None,
                           ..Default::default()
                       });

                       custom_gui_commands = gui.renderer.paint_with::<GameObjectPaint>(
                           &mut gui.yak,
                           &self.gpu.device,
                           &self.gpu.queue,
                           &mut render_pass,
                           surface_info,
                           &mut resources,
                       );
                   }

                   self.render_resources.gui_resources = resources.gui_resources.take();

                   self.gui_animation_matrix_data_map = resources.animation_matrix_data_map.take();
                   self.gui_opaque_draws = resources.opaque_draws.take();
                   self.gui_non_opaque_draws = resources.non_opaque_draws.take();

                   self.gui_packed_size = resources.packed_size;
                   self.gui_rects = resources.rects;

                   self.animation_cache = resources.animation_cache;
               };
        */

        let surface_view = surface.texture.create_view(&view_desc);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&res.present_res.game_gui_compose_pipeline.render_pipeline);
            render_pass.set_bind_group(
                0,
                &res.present_res.game_gui_compose_pipeline.bind_group,
                &[],
            );
            render_pass.draw(0..3, 0..1)
        }

        let screenshot_buffer = if screenshotting {
            let texture = res.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Screenshot Texture"),
                size: surface_dim,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SCREENSHOT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            wgpu::util::TextureBlitter::new(&res.device, SCREENSHOT_FORMAT).copy(
                &res.device,
                &mut encoder,
                &surface_view,
                &texture.create_view(&view_desc),
            );

            let buffer = res.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Screenshot Buffer"),
                size: surface_buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(surface_buffer_dim.width * surface_pixel_size),
                        rows_per_image: Some(surface_buffer_dim.height),
                    },
                },
                surface_buffer_dim,
            );

            Some(buffer)
        } else {
            None
        };

        res.queue.submit([encoder.finish()]);

        if let Some(buffer) = screenshot_buffer {
            let slice = buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, move |result| {
                result.unwrap();
            });

            res.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            let padded_data = slice.get_mapped_range().to_vec();
            let mut data = Vec::new();

            let padded_width = (surface_buffer_dim.width * surface_pixel_size) as usize;
            let unpadded_width = (surface_dim.width * surface_pixel_size) as usize;

            for chunk in padded_data.chunks(padded_width) {
                data.extend(&chunk[..unpadded_width]);
            }

            if let Some(image) = image::RgbaImage::from_vec(size.width, size.height, data) {
                use image::EncodableLayout;

                self.screenshot_clipboard
                    .set_image(arboard::ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        bytes: Cow::from(image.as_bytes()),
                    })
                    .unwrap();
            }
        }

        res.window.pre_present_notify();
        surface.present();

        Ok(())
    }
}
