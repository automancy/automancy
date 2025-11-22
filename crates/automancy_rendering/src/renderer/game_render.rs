use std::{collections::VecDeque, time::Instant};

use automancy_data::{
    game::coord::TileCoord,
    id::{Id, ModelId, RenderId},
    math::{Float, Vec2},
    rendering,
    rendering::{
        colors::Rgba,
        draw::{GameDrawInstance, GameUniformData, NO_COLOR_OFFSET},
    },
};
use automancy_game::{actor::message::GameMsg, scripting::render::RenderCommand, state::AutomancyGameState};
use hashbrown::{HashMap, HashSet};
use yakui::{Rect, UVec2};

use crate::{
    gpu,
    gpu::{
        RenderResources,
        data::{GpuGameLightingUniformData, GpuGameUniformData, GpuPostProcessingUniformData},
    },
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
    draw_ids: HashMap<TileCoord, HashSet<(RenderId, ModelId)>>,

    pub tile_tints: HashMap<TileCoord, Rgba>,
    last_tile_tints: HashMap<TileCoord, Rgba>,

    pub take_item_animations: HashMap<Id, VecDeque<(Instant, Rect)>>,

    gui_packed_size: Option<UVec2>,
    gui_rects: Vec<Option<crunch::Rect>>,
}

impl GameRenderer {
    pub fn new() -> Self {
        Self {
            draw_ids: Default::default(),

            tile_tints: Default::default(),
            last_tile_tints: Default::default(),

            take_item_animations: Default::default(),

            gui_packed_size: Default::default(),
            gui_rects: Default::default(),
        }
    }

    #[inline]
    pub fn render(
        &mut self,
        surface: &wgpu::SurfaceTexture,
        game_state: &AutomancyGameState,
        render_state: &mut AutomancyRenderState,
        res: &mut RenderResources,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let last_tile_tints = std::mem::take(&mut self.last_tile_tints);
        let tile_tints = std::mem::take(&mut self.tile_tints);

        let render_commands = {
            let culling_bounds = game_state.camera.culling_bounds;
            let game_handle = game_state.game_handle.clone();

            game_state
                .tokio
                .block_on(game_handle.call(|reply| GameMsg::GetAllRenderCommands { culling_bounds, reply }, None))
                .unwrap()
                .unwrap()
        };

        let world_matrix = game_state.camera.get_matrix();
        render_state.instance_man.set_all_world_matrix(world_matrix);

        for batch in render_commands {
            for (coord, commands) in batch {
                for command in commands {
                    match command {
                        RenderCommand::Track { render_id, model_id } => {
                            self.draw_ids.entry(coord).or_default().insert((render_id, model_id));

                            render_state.instance_man.insert(
                                &game_state.resource_man,
                                &render_state.model_man,
                                (coord, render_id, model_id),
                                GameDrawInstance {
                                    world_matrix,
                                    ..Default::default()
                                },
                            );
                        }
                        RenderCommand::Untrack { render_id, model_id } => {
                            self.draw_ids.entry(coord).or_default().remove(&(render_id, model_id));

                            render_state.instance_man.remove((coord, render_id, model_id));
                        }
                        RenderCommand::Transform {
                            render_id,
                            model_id,
                            model_matrix,
                        } => {
                            render_state.instance_man.set_matrix(
                                &game_state.resource_man,
                                &render_state.model_man,
                                (coord, render_id, model_id),
                                (Some(model_matrix), None),
                            );
                        }
                    }
                }
            }

            render_state.instance_man.flush(&game_state.resource_man, &render_state.model_man);
        }

        for (&coord, _) in &last_tile_tints {
            if tile_tints.contains_key(&coord) {
                continue;
            };

            let Some(ids) = self.draw_ids.get(&coord) else {
                continue;
            };

            for &(render_id, model_id) in ids {
                render_state.instance_man.modify_instances(
                    &game_state.resource_man,
                    &render_state.model_man,
                    (coord, render_id, model_id),
                    |_, instance| {
                        instance.color_offset = NO_COLOR_OFFSET.into_array();
                    },
                );
            }
        }

        for (&coord, &tint) in &tile_tints {
            let Some(ids) = self.draw_ids.get(&coord) else {
                continue;
            };

            for &(render_id, model_id) in ids {
                render_state.instance_man.modify_instances(
                    &game_state.resource_man,
                    &render_state.model_man,
                    (coord, render_id, model_id),
                    |_, instance| {
                        instance.color_offset = tint.into_array();
                    },
                );
            }
        }

        self.last_tile_tints = tile_tints;

        let [opaque_draws, non_opaque_draws] = render_state.instance_man.collect_draw_calls(res, &render_state.model_man);

        gpu::util::upload_buffer_or_recreate(
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.opaque_indirect_draw_command_buffer,
            &opaque_draws,
        );
        gpu::util::upload_buffer_or_recreate(
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.non_opaque_indirect_draw_command_buffer,
            &non_opaque_draws,
        );

        render_state
            .instance_man
            .upload_animation(res, game_state.start_instant, &render_state.model_man);
        res.queue.submit([]);

        self.inner_render(surface, game_state, res, encoder, opaque_draws, non_opaque_draws)
    }
}

impl Default for GameRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRenderer {
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn inner_render(
        &mut self,
        surface: &wgpu::SurfaceTexture,
        game_state: &AutomancyGameState,
        res: &mut RenderResources,
        encoder: &mut wgpu::CommandEncoder,
        opaque_draws: Vec<wgpu::util::DrawIndexedIndirectArgs>,
        non_opaque_draws: Vec<wgpu::util::DrawIndexedIndirectArgs>,
    ) {
        let surface_size = surface.texture.size();

        let game_uniform = GameUniformData {
            camera_pos: game_state.camera.get_pos(),
            camera_bounds: rendering::view::viewport_bounding_rect_in_world(
                Vec2::new(surface_size.width as Float, surface_size.height as Float),
                game_state.camera.get_pos(),
            ),
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
                            .create_view(&wgpu::TextureViewDescriptor::default()),
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
                            .create_view(&wgpu::TextureViewDescriptor::default()),
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
                            .create_view(&wgpu::TextureViewDescriptor::default()),
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
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
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
            render_pass.set_index_buffer(res.global_res.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            if !opaque_draws.is_empty() {
                render_pass.set_vertex_buffer(1, res.main_game_res.game_pipeline.opaque_instance_buffer.slice(..));
                render_pass.multi_draw_indexed_indirect(
                    &res.main_game_res.game_pipeline.opaque_indirect_draw_command_buffer,
                    0,
                    opaque_draws.len() as u32,
                );
            }

            if !non_opaque_draws.is_empty() {
                render_pass.set_vertex_buffer(1, res.main_game_res.game_pipeline.non_opaque_instance_buffer.slice(..));
                render_pass.multi_draw_indexed_indirect(
                    &res.main_game_res.game_pipeline.non_opaque_indirect_draw_command_buffer,
                    0,
                    non_opaque_draws.len() as u32,
                );
            }
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
                        .create_view(&wgpu::TextureViewDescriptor::default()),
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
            render_pass.set_bind_group(0, &res.main_game_res.game_lighting_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &res.main_game_res.game_lighting_pipeline.bind_group_samplers, &[]);
            render_pass.set_bind_group(2, &res.main_game_res.game_lighting_pipeline.bind_group_textures, &[]);
            render_pass.draw(0..3, 0..1);
        }

        {
            res.queue.write_buffer(
                &res.main_game_res.post_processing_pipeline.uniform_buffer,
                0,
                bytemuck::cast_slice(&[GpuPostProcessingUniformData { ..Default::default() }]),
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Post Processing Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res
                        .main_game_res
                        .render_textures
                        .post_processing_surface_texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
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
            render_pass.set_bind_group(0, &res.main_game_res.post_processing_pipeline.bind_group_uniform, &[]);
            render_pass.set_bind_group(1, &res.main_game_res.post_processing_pipeline.bind_group_samplers, &[]);
            render_pass.set_bind_group(2, &res.main_game_res.post_processing_pipeline.bind_group_textures, &[]);
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
                        .create_view(&wgpu::TextureViewDescriptor::default()),
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
                    view: &res.gui_res.gui_texture.create_view(&wgpu::TextureViewDescriptor::default()),
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
;

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res.present_res.present_texture.create_view(&wgpu::TextureViewDescriptor::default()),
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
            render_pass.set_bind_group(0, &res.present_res.game_gui_compose_pipeline.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}
