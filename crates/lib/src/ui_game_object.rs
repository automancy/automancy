use crate::gpu::{self, MODEL_DEPTH_CLEAR, NORMAL_CLEAR};
use crate::renderer::{try_add_animation, YakuiRenderResources};
use automancy_defs::coord::TileCoord;
use automancy_defs::rendering::{
    AnimationMatrixData, GameMatrix, GameUBO, GpuInstance, IntermediateUBO, MatrixData,
    WorldMatrixData,
};
use automancy_resources::rhai_render::RenderCommand;
use automancy_system::tile_entity::collect_render_commands;
use automancy_ui::{GameElementPaint, UiGameObjectType, SHOULD_RERENDER};
use core::cell::Cell;
use std::time::Instant;
use wgpu::util::{BufferInitDescriptor, DeviceExt, DrawIndexedIndirectArgs};
use wgpu::BindGroupEntry;
use wgpu::{
    BindGroupDescriptor, BindingResource, BufferUsages, Color, IndexFormat, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
    TextureViewDescriptor,
};
use yakui::UVec2;
use yakui_wgpu::CallbackTrait;

thread_local! {
    static START_INSTANT: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub fn init_custom_paint_state(start_instant: Instant) {
    START_INSTANT.set(Some(start_instant));
}

impl CallbackTrait<YakuiRenderResources> for GameElementPaint {
    fn prepare(&mut self, YakuiRenderResources { instances, .. }: &mut YakuiRenderResources) {
        let props = &self.props;

        instances.as_mut().unwrap().push((
            props.ty.clone(),
            props.instance,
            GameMatrix::<false>::new(props.model_matrix, props.world_matrix),
            (props.index, props.size),
        ));
    }

    fn finish_prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        YakuiRenderResources {
            resource_man,
            global_resources,
            gui_resources,
            surface_format,
            animation_cache,
            animation_matrix_data_map,
            opaque_draws: opaque_draw_info,
            non_opaque_draws: non_opaque_draw_info,
            instances,
            packed_size,
            rects,
        }: &mut YakuiRenderResources,
    ) {
        let gui_resources = gui_resources.as_mut().unwrap();
        let start_instant = START_INSTANT.get().unwrap();

        if let Some(instances) = instances.take() {
            let items = instances
                .iter()
                .map(|(.., (index, size))| {
                    crunch::Item::new(
                        *index,
                        (size.x.round() * 2.0) as usize,
                        (size.y.round() * 2.0) as usize,
                        crunch::Rotation::None,
                    )
                })
                .collect::<Vec<_>>();

            let packed =
                crunch::pack_into_po2(device.limits().max_texture_dimension_2d as usize, items)
                    .expect("gui game objects exceed max texture size.");

            let size = UVec2::new(packed.w as u32, packed.h as u32);

            let mut opaque_draw_info = opaque_draw_info.as_mut().unwrap();
            let mut non_opaque_draw_info = non_opaque_draw_info.as_mut().unwrap();
            let animation_matrix_data_map = animation_matrix_data_map.as_mut().unwrap();

            if SHOULD_RERENDER.get() {
                rects.clear();

                let mut gpu_instances = vec![];
                let mut matrix_data = vec![];
                let mut world_matrix_data = vec![];

                opaque_draw_info.clear();
                non_opaque_draw_info.clear();
                animation_matrix_data_map.clear();

                for (ty, instance, game_matrix, (rect_index, _)) in instances.into_iter() {
                    let models = match ty {
                        UiGameObjectType::Tile(tile_id, mut data) => {
                            if let Some(commands) = collect_render_commands(
                                resource_man,
                                tile_id,
                                TileCoord::ZERO,
                                &mut data,
                                &mut Default::default(),
                                true,
                                false,
                            ) {
                                commands
                                    .into_iter()
                                    .flat_map(|v| match v {
                                        RenderCommand::Track { model, .. } => Some(model),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                vec![]
                            }
                        }
                        UiGameObjectType::Model(model_id) => vec![model_id],
                    };

                    for model in models {
                        let (model, (meshes, ..)) = resource_man.mesh_or_missing_tile_mesh(&model);

                        world_matrix_data.push(WorldMatrixData::new(game_matrix.world_matrix()));

                        for mesh in meshes.iter().flatten() {
                            let draw_info = if mesh.opaque {
                                &mut opaque_draw_info
                            } else {
                                &mut non_opaque_draw_info
                            };

                            let (animation_matrix_index, ..) = animation_matrix_data_map
                                .insert_full((model, mesh.index), AnimationMatrixData::default());

                            matrix_data
                                .push(MatrixData::new(game_matrix.model_matrix(), mesh.matrix));

                            gpu_instances.push(GpuInstance {
                                matrix_index: (matrix_data.len() - 1) as u32,
                                world_matrix_index: (world_matrix_data.len() - 1) as u32,
                                animation_matrix_index: animation_matrix_index as u32,
                                color_offset: instance.color_offset,
                                alpha: instance.alpha,
                            });

                            let index_range = &resource_man.all_index_ranges[&model][&mesh.index];

                            draw_info.push((
                                DrawIndexedIndirectArgs {
                                    first_index: index_range.pos,
                                    index_count: index_range.count,
                                    base_vertex: index_range.base_vertex,
                                    first_instance: (gpu_instances.len() - 1) as u32,
                                    instance_count: 1,
                                },
                                rect_index,
                            ));
                        }
                    }
                }

                gpu::resize_update_buffer(
                    device,
                    queue,
                    &mut gui_resources.instance_buffer,
                    &gpu_instances,
                );

                queue.write_buffer(
                    &gui_resources.matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(&matrix_data),
                );

                queue.write_buffer(
                    &gui_resources.world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(&world_matrix_data),
                );

                queue.write_buffer(
                    &gui_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::default()]),
                );

                if *packed_size != Some(size) {
                    gui_resources.resize(device, *surface_format, global_resources, size);
                }
                *packed_size = Some(size);

                for item in packed.items.iter() {
                    if item.data >= rects.len() {
                        rects.resize(item.data + 1, None);
                    }

                    rects[item.data] = Some(item.rect);
                }
            }

            for (model, _) in animation_matrix_data_map.keys() {
                try_add_animation(resource_man, start_instant, *model, animation_cache);
            }

            for (&model, anim) in animation_cache.iter() {
                for (&mesh_id, &matrix) in anim {
                    if let Some(data) = animation_matrix_data_map.get_mut(&(model, mesh_id)) {
                        data.animation_matrix = matrix.to_cols_array_2d();
                    }
                }
            }

            gpu::ordered_map_update_buffer(
                queue,
                &gui_resources.animation_matrix_data_buffer,
                animation_matrix_data_map,
            );

            {
                let color = gui_resources
                    .color_texture()
                    .create_view(&TextureViewDescriptor::default());
                let depth = gui_resources
                    .depth_texture()
                    .create_view(&TextureViewDescriptor::default());
                let normal = gui_resources
                    .normal_texture()
                    .create_view(&TextureViewDescriptor::default());
                let model_depth = gui_resources
                    .model_depth_texture()
                    .create_view(&TextureViewDescriptor::default());

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Render Pass"),
                    color_attachments: &[
                        Some(RenderPassColorAttachment {
                            view: &color,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        }),
                        Some(RenderPassColorAttachment {
                            view: &normal,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(NORMAL_CLEAR),
                                store: StoreOp::Store,
                            },
                        }),
                        Some(RenderPassColorAttachment {
                            view: &model_depth,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(MODEL_DEPTH_CLEAR),
                                store: StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &depth,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });

                render_pass.set_pipeline(&global_resources.game_pipeline);
                render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
                render_pass.set_vertex_buffer(0, global_resources.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, gui_resources.instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(global_resources.index_buffer.slice(..), IndexFormat::Uint16);

                for (draw, rect_index) in opaque_draw_info {
                    let rect = rects[*rect_index].unwrap();

                    render_pass.set_viewport(
                        rect.x as f32,
                        rect.y as f32,
                        rect.w as f32,
                        rect.h as f32,
                        0.0,
                        1.0,
                    );

                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }

                for (draw, rect_index) in non_opaque_draw_info {
                    let rect = rects[*rect_index].unwrap();

                    render_pass.set_viewport(
                        rect.x as f32,
                        rect.y as f32,
                        rect.w as f32,
                        rect.h as f32,
                        0.0,
                        1.0,
                    );

                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }
            }

            {
                let view = gui_resources
                    .post_processing_texture()
                    .create_view(&TextureViewDescriptor::default());

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Post Processing Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&global_resources.post_processing_pipeline);
                render_pass.set_bind_group(
                    0,
                    gui_resources.post_processing_bind_group_textures(),
                    &[],
                );
                render_pass.set_bind_group(
                    1,
                    &gui_resources.post_processing_bind_group_uniform,
                    &[],
                );
                render_pass.draw(0..3, 0..1);
            }

            {
                let view = gui_resources
                    .present_texture()
                    .create_view(&TextureViewDescriptor::default());

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Antialiasing Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&global_resources.fxaa_pipeline);
                render_pass.set_bind_group(0, gui_resources.antialiasing_bind_group(), &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        if self.present_uniform.is_none() {
            self.present_uniform = Some(device.create_buffer_init(&BufferInitDescriptor {
                label: Some("UI Model Present Uniform Buffer"),
                contents: bytemuck::cast_slice(&[IntermediateUBO {
                    viewport_size: [0.0; 2],
                    viewport_pos: [0.0; 2],
                }]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }));
        }

        if let Some((Some(rect), packed_size)) =
            rects.get(self.props.index).cloned().zip(*packed_size)
        {
            queue.write_buffer(
                self.present_uniform.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&[IntermediateUBO {
                    viewport_size: [
                        (rect.w as f32 * self.clip_scale.x) / packed_size.x as f32,
                        (rect.h as f32 * self.clip_scale.y) / packed_size.y as f32,
                    ],
                    viewport_pos: [
                        (rect.x as f32 + self.clip_offset.x) / packed_size.x as f32,
                        (rect.y as f32 + self.clip_offset.y) / packed_size.y as f32,
                    ],
                }]),
            );
        }

        if self.present_bind_group.is_none() {
            self.present_bind_group = Some(
                device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &global_resources.intermediate_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(
                                &gui_resources
                                    .present_texture()
                                    .create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(
                                &global_resources.nonfiltering_sampler,
                            ),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: self.present_uniform.as_ref().unwrap().as_entire_binding(),
                        },
                    ],
                }),
            );
        }
    }

    fn paint<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        YakuiRenderResources {
            global_resources, ..
        }: &'a YakuiRenderResources,
    ) {
        if let Some(present_bind_group) = self.present_bind_group.as_ref() {
            render_pass.set_pipeline(&global_resources.multisampled_present_pipeline);
            render_pass.set_bind_group(0, present_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}
