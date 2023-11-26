use std::borrow::Cow;
use std::f32::consts::FRAC_PI_2;
use std::time::Instant;

use arboard::{Clipboard, ImageData};
use egui::{Rect, Rgba};
use egui_wgpu::renderer::ScreenDescriptor;
use futures::executor::block_on;
use image::{EncodableLayout, RgbaImage};
use num::PrimInt;
use tokio::sync::oneshot;
use egui_wgpu::wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, BufferAddress, BufferDescriptor,
    BufferUsages, Color, CommandEncoderDescriptor, ImageCopyBuffer, ImageDataLayout, IndexFormat,
    LoadOp, Maintain, MapMode, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, SurfaceError, TextureDescriptor,
    TextureDimension, TextureUsages, TextureViewDescriptor, COPY_BUFFER_ALIGNMENT,
    COPY_BYTES_PER_ROW_ALIGNMENT,
};

use automancy::game::{GameMsg, RenderUnit, TransactionRecord, TRANSACTION_ANIMATION_SPEED};
use automancy::gpu;
use automancy::gpu::{Gpu, NORMAL_CLEAR, SCREENSHOT_FORMAT};
use automancy::input::KeyActions;
use automancy_defs::cgmath::{point3, vec3, EuclideanSpace};
use automancy_defs::coord::TileCoord;
use automancy_defs::gui::Gui;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_defs::math::{deg, direction_to_angle, Float, Matrix4, FAR};
use automancy_defs::rendering::{
    lerp_coords_to_pixel, make_line, GameUBO, InstanceData, PostEffectsUBO,
};
use automancy_defs::{bytemuck, colors, math};
use automancy_resources::data::Data;

use crate::setup::GameSetup;

pub struct Renderer {
    pub gpu: Gpu,
}

impl Renderer {
    pub fn new(gpu: Gpu) -> Self {
        Self { gpu }
    }
}

fn get_angle_from_direction(target: &Data) -> Option<Float> {
    if let Some(target) = target.as_coord() {
        match *target {
            TileCoord::TOP_RIGHT => Some(0.0),
            TileCoord::RIGHT => Some(-60.0),
            TileCoord::BOTTOM_RIGHT => Some(-120.0),
            TileCoord::BOTTOM_LEFT => Some(-180.0),
            TileCoord::LEFT => Some(-240.0),
            TileCoord::TOP_LEFT => Some(-300.0),
            _ => None,
        }
    } else {
        None
    }
}

pub type GuiInstances = Vec<(InstanceData, Id, (Option<Rect>, Option<Rect>))>;

impl Renderer {
    pub fn render(
        &mut self,
        setup: &GameSetup,
        gui: &mut Gui,
        matrix: Matrix4,
        tile_tints: HashMap<TileCoord, Rgba>,
        mut extra_instances: Vec<(InstanceData, Id)>,
        mut overlay_instances: Vec<(InstanceData, Id)>,
        mut in_world_item_instances: Vec<(InstanceData, Id)>,
        mut gui_instances: GuiInstances,
        mut item_instances: GuiInstances,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();

        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let culling_range = setup.camera.culling_range;
        let camera_pos = setup.camera.get_pos();
        let camera_pos_float = camera_pos.cast::<Float>().unwrap();

        let map_render_info = block_on(setup.game.call(
            |reply| GameMsg::RenderInfoRequest {
                culling_range,
                reply,
            },
            None,
        ))
        .unwrap()
        .unwrap();

        let (mut instances, all_data) = map_render_info;

        for (coord, instance) in instances.iter_mut() {
            if let Some(theta) = all_data
                .get(coord)
                .and_then(|data| data.get(&setup.resource_man.registry.data_ids.target))
                .and_then(get_angle_from_direction)
            {
                let m = &mut instance.instance.model_matrix;

                *m = *m * Matrix4::from_angle_z(deg(theta))
            } else if let Some(inactive) = setup
                .resource_man
                .registry
                .tile_data(
                    instance.tile,
                    setup.resource_man.registry.data_ids.inactive_model,
                )
                .and_then(Data::as_id)
            {
                instance.model = setup.resource_man.get_model(*inactive);
            }
        }

        for (coord, data) in all_data {
            if let Some(link) = data
                .get(&setup.resource_man.registry.data_ids.link)
                .and_then(Data::as_coord)
                .cloned()
            {
                extra_instances.push((
                    InstanceData {
                        color_offset: colors::RED.to_array(),
                        light_pos: camera_pos_float,
                        model_matrix: make_line(
                            math::hex_to_pixel(*coord),
                            math::hex_to_pixel(*link),
                        ),
                        ..Default::default()
                    },
                    setup.resource_man.registry.model_ids.cube1x1,
                ));
            }
        }

        let transaction_records_mutex =
            block_on(setup.game.call(GameMsg::GetRecordedTransactions, None))
                .unwrap()
                .unwrap();
        let transaction_records = transaction_records_mutex.lock().unwrap();
        let now = Instant::now();

        for ((source_coord, coord), instants) in transaction_records.iter() {
            if culling_range.contains(*source_coord) && culling_range.contains(*coord) {
                for (instant, TransactionRecord { stack, .. }) in instants {
                    let duration = now.duration_since(*instant);
                    let t = duration.as_secs_f64() / TRANSACTION_ANIMATION_SPEED.as_secs_f64();

                    let point = lerp_coords_to_pixel(*source_coord, *coord, t);

                    let direction = *coord - *source_coord;
                    let direction = math::hex_to_pixel(direction.into());
                    let theta = direction_to_angle(direction.to_vec());

                    let instance = InstanceData::default()
                        .with_model_matrix(
                            Matrix4::from_translation(vec3(
                                point.x as Float,
                                point.y as Float,
                                FAR as Float,
                            )) * Matrix4::from_angle_z(theta)
                                * Matrix4::from_scale(0.3),
                        )
                        .with_light_pos(camera_pos_float);
                    let model = setup.resource_man.get_item_model(stack.item);

                    in_world_item_instances.push((instance, model));
                }
            }
        }

        let mut game_instances = {
            let none = setup
                .resource_man
                .registry
                .tile(setup.resource_man.registry.none)
                .unwrap()
                .models[0];

            for q in culling_range.start().q()..=culling_range.end().q() {
                for r in culling_range.start().r()..=culling_range.end().r() {
                    let coord = TileCoord::new(q, r);

                    if !instances.contains_key(&coord) {
                        let p = math::hex_to_pixel(coord.into());

                        instances.insert(
                            coord,
                            RenderUnit {
                                instance: InstanceData::default().with_model_matrix(
                                    Matrix4::from_translation(vec3(
                                        p.x as Float,
                                        p.y as Float,
                                        FAR as Float,
                                    )),
                                ),
                                tile: none,
                                model: none,
                            },
                        );
                    }
                }
            }

            for (coord, color) in tile_tints.into_iter() {
                if let Some(RenderUnit { instance, .. }) = instances.get_mut(&coord) {
                    *instance = instance.with_color_offset(color.to_array())
                }
            }

            let mut map = HashMap::new();

            for RenderUnit {
                instance, model, ..
            } in instances.into_values()
            {
                map.entry(model)
                    .or_insert_with(|| Vec::with_capacity(32))
                    .push((instance.with_light_pos(camera_pos_float), model, ()))
            }

            map.into_values().flatten().collect::<Vec<_>>()
        };

        extra_instances.sort_by_key(|v| v.1);
        let mut extra_instances = extra_instances
            .into_iter()
            .map(|(instance, id)| (instance, id, ()))
            .collect::<Vec<_>>();
        game_instances.append(&mut extra_instances);

        overlay_instances.sort_by_key(|v| v.1);
        let overlay_instances = overlay_instances
            .into_iter()
            .map(|(instance, id)| (instance, id, ()))
            .collect::<Vec<_>>();

        in_world_item_instances.sort_by_key(|v| v.1);
        let in_world_item_instances = in_world_item_instances
            .into_iter()
            .map(|(instance, id)| (instance, id, ()))
            .collect::<Vec<_>>();

        gui_instances.sort_by_key(|v| v.1);
        item_instances.sort_by_key(|v| v.1);

        self.inner_render(
            setup,
            gui,
            matrix,
            &game_instances,
            &overlay_instances,
            &in_world_item_instances,
            &gui_instances,
            &item_instances,
        )
    }

    fn inner_render(
        &mut self,
        setup: &GameSetup,
        gui: &mut Gui,
        matrix: Matrix4,
        game_instances: &[(InstanceData, Id, ())],
        overlay_instances: &[(InstanceData, Id, ())],
        in_world_item_instances: &[(InstanceData, Id, ())],
        gui_instances: &GuiInstances,
        item_instances: &GuiInstances,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();
        let factor = gui.context.pixels_per_point();

        let (game_instances, game_draws, game_draw_count) =
            gpu::indirect_instance(&setup.resource_man, game_instances, true);

        let (in_world_item_instances, in_world_item_draws, in_world_item_draw_count) =
            gpu::indirect_instance(&setup.resource_man, in_world_item_instances, true);

        let egui_out = gui.context.end_frame();
        let egui_primitives = gui.context.tessellate(egui_out.shapes, gui.context.pixels_per_point());
        let egui_desc = ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: factor,
        };

        let (gui_instances, gui_draws, gui_draw_count) =
            gpu::indirect_instance(&setup.resource_man, gui_instances, false);

        let (item_instances, item_draws, item_draw_count) =
            gpu::indirect_instance(&setup.resource_man, item_instances, false);

        let (overlay_instances, overlay_draws, overlay_draw_count) =
            gpu::indirect_instance(&setup.resource_man, overlay_instances, true);

        let output = self.gpu.surface.get_current_texture()?;

        {
            let output_size = output.texture.size();

            if output_size.width != size.width || output_size.height != size.height {
                return Ok(());
            }
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.game_resources.instance_buffer,
                bytemuck::cast_slice(game_instances.as_slice()),
            );
            let mut indirect_buffer = vec![];
            game_draws
                .into_iter()
                .flat_map(|v| v.1)
                .for_each(|v| indirect_buffer.extend_from_slice(v.0.as_bytes()));
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.game_resources.indirect_buffer,
                indirect_buffer.as_slice(),
            );

            let mut game_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(NORMAL_CLEAR),
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if game_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.game_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(matrix)]),
                );

                game_pass.set_viewport(
                    0.0,
                    0.0,
                    size.width as Float,
                    size.height as Float,
                    1.0,
                    0.0,
                );
                game_pass.set_pipeline(&self.gpu.game_resources.pipeline);
                game_pass.set_bind_group(0, &self.gpu.game_resources.bind_group, &[]);
                game_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                game_pass.set_vertex_buffer(1, self.gpu.game_resources.instance_buffer.slice(..));
                game_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                game_pass.multi_draw_indexed_indirect(
                    &self.gpu.game_resources.indirect_buffer,
                    0,
                    game_draw_count,
                );
            }
        }

        {
            let mut post_effects_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Post Effects Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.post_effects_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.gpu.queue.write_buffer(
                &self.gpu.game_resources.post_effects_uniform_buffer,
                0,
                bytemuck::cast_slice(&[PostEffectsUBO::default()]),
            );

            post_effects_pass.set_pipeline(&self.gpu.post_effects_resources.pipeline);
            post_effects_pass.set_bind_group(
                0,
                self.gpu.game_resources.post_effects_bind_group(),
                &[],
            );
            post_effects_pass.draw(0..3, 0..1);
        }

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.in_world_item_resources.instance_buffer,
                bytemuck::cast_slice(in_world_item_instances.as_slice()),
            );
            let mut indirect_buffer = vec![];
            in_world_item_draws
                .into_iter()
                .flat_map(|v| v.1)
                .for_each(|v| indirect_buffer.extend_from_slice(v.0.as_bytes()));
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.in_world_item_resources.indirect_buffer,
                indirect_buffer.as_slice(),
            );

            let mut in_world_item_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("In-world Item Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.post_effects_resources.texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: false,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: false,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            if in_world_item_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.in_world_item_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(matrix)]),
                );

                in_world_item_pass.set_viewport(
                    0.0,
                    0.0,
                    (size.width) as Float,
                    (size.height) as Float,
                    1.0,
                    0.0,
                );
                in_world_item_pass.set_pipeline(&self.gpu.game_resources.pipeline);
                in_world_item_pass.set_bind_group(0, &self.gpu.game_resources.bind_group, &[]);
                in_world_item_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                in_world_item_pass.set_vertex_buffer(
                    1,
                    self.gpu.in_world_item_resources.instance_buffer.slice(..),
                );
                in_world_item_pass
                    .set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                in_world_item_pass.multi_draw_indexed_indirect(
                    &self.gpu.in_world_item_resources.indirect_buffer,
                    0,
                    in_world_item_draw_count,
                );
            }
        }

        {
            let mut antialiasing_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Antialiasing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.antialiasing_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            antialiasing_pass.set_pipeline(&self.gpu.antialiasing_resources.pipeline);
            antialiasing_pass.set_bind_group(
                0,
                self.gpu.game_resources.antialiasing_bind_group(),
                &[],
            );
            antialiasing_pass.draw(0..3, 0..1);
        }

        let user_commands = {
            let user_commands = {
                for (id, delta) in egui_out.textures_delta.set {
                    gui.renderer
                        .update_texture(&self.gpu.device, &self.gpu.queue, id, &delta);
                }

                gui.renderer.update_buffers(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut encoder,
                    &egui_primitives,
                    &egui_desc,
                )
            };

            {
                let mut egui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Egui Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &self.gpu.egui_resources.texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                gui.renderer
                    .render(&mut egui_pass, &egui_primitives, &egui_desc);
            }

            for id in &egui_out.textures_delta.free {
                gui.renderer.free_texture(id);
            }

            user_commands
        };

        {
            let mut combine_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Combine Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.first_combine_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            combine_pass.set_pipeline(&self.gpu.first_combine_resources.pipeline);
            combine_pass.set_bind_group(0, self.gpu.first_combine_resources.bind_group(), &[]);
            combine_pass.draw(0..3, 0..1)
        }

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.gui_resources.instance_buffer,
                bytemuck::cast_slice(gui_instances.as_slice()),
            );

            let mut gui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(NORMAL_CLEAR),
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if gui_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.gui_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(math::matrix(
                        point3(0.0, 0.0, 3.0),
                        1.0,
                        FRAC_PI_2,
                    ))]),
                );

                gui_pass.set_pipeline(&self.gpu.game_resources.pipeline);
                gui_pass.set_bind_group(0, &self.gpu.gui_resources.bind_group, &[]);
                gui_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                gui_pass.set_vertex_buffer(1, self.gpu.gui_resources.instance_buffer.slice(..));
                gui_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                for (draw, (viewport, scissor)) in gui_draws.values().flatten() {
                    if let Some(viewport) = viewport {
                        gui_pass.set_viewport(
                            viewport.left() * factor as Float,
                            viewport.top() * factor as Float,
                            viewport.width() * factor as Float,
                            viewport.height() * factor as Float,
                            1.0,
                            0.0,
                        );
                    } else {
                        gui_pass.set_viewport(
                            0.0,
                            0.0,
                            (size.width) as Float,
                            (size.height) as Float,
                            1.0,
                            0.0,
                        );
                    }

                    if let Some(scissor) = scissor {
                        gui_pass.set_scissor_rect(
                            (scissor.left() * factor) as u32,
                            (scissor.top() * factor) as u32,
                            (scissor.width() * factor) as u32,
                            (scissor.height() * factor) as u32,
                        );
                    } else {
                        gui_pass.set_scissor_rect(0, 0, size.width, size.height);
                    }

                    gui_pass.draw_indexed(
                        draw.base_index..(draw.base_index + draw.vertex_count),
                        draw.vertex_offset,
                        draw.base_instance..(draw.base_instance + draw.instance_count),
                    );
                }
            }
        }

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.overlay_resources.instance_buffer,
                bytemuck::cast_slice(overlay_instances.as_slice()),
            );
            let mut indirect_buffer = vec![];
            overlay_draws
                .into_iter()
                .flat_map(|v| v.1)
                .for_each(|v| indirect_buffer.extend_from_slice(v.0.as_bytes()));
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.overlay_resources.indirect_buffer,
                indirect_buffer.as_slice(),
            );

            let mut overlay = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Overlay Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if overlay_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.overlay_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(matrix)]),
                );

                overlay.set_viewport(
                    0.0,
                    0.0,
                    (size.width) as Float,
                    (size.height) as Float,
                    1.0,
                    0.0,
                );
                overlay.set_pipeline(&self.gpu.game_resources.pipeline);
                overlay.set_bind_group(0, &self.gpu.game_resources.bind_group, &[]);
                overlay.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                overlay.set_vertex_buffer(1, self.gpu.overlay_resources.instance_buffer.slice(..));
                overlay.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                overlay.multi_draw_indexed_indirect(
                    &self.gpu.overlay_resources.indirect_buffer,
                    0,
                    overlay_draw_count,
                );
            }
        }

        {
            let mut post_effects_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Post Effects Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.post_effects_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.gpu.queue.write_buffer(
                &self.gpu.gui_resources.post_effects_uniform_buffer,
                0,
                bytemuck::cast_slice(&[PostEffectsUBO::default()]),
            );

            post_effects_pass.set_pipeline(&self.gpu.post_effects_resources.pipeline);
            post_effects_pass.set_bind_group(
                0,
                self.gpu.gui_resources.post_effects_bind_group(),
                &[],
            );
            post_effects_pass.draw(0..3, 0..1);
        }

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.item_resources.instance_buffer,
                bytemuck::cast_slice(item_instances.as_slice()),
            );

            let mut item_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Item Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.post_effects_resources.texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: false,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gpu.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: false,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            if item_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.item_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::default()]),
                );

                item_pass.set_pipeline(&self.gpu.game_resources.pipeline);
                item_pass.set_bind_group(0, &self.gpu.item_resources.bind_group, &[]);
                item_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                item_pass.set_vertex_buffer(1, self.gpu.item_resources.instance_buffer.slice(..));
                item_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                for (draw, (viewport, scissor)) in item_draws.values().flatten() {
                    if let Some(viewport) = viewport {
                        item_pass.set_viewport(
                            viewport.left() * factor as Float,
                            viewport.top() * factor as Float,
                            viewport.width() * factor as Float,
                            viewport.height() * factor as Float,
                            0.0,
                            1.0,
                        );
                    } else {
                        item_pass.set_viewport(
                            0.0,
                            0.0,
                            (size.width) as Float,
                            (size.height) as Float,
                            0.0,
                            1.0,
                        );
                    }

                    if let Some(scissor) = scissor {
                        item_pass.set_scissor_rect(
                            (scissor.left() * factor) as u32,
                            (scissor.top() * factor) as u32,
                            (scissor.width() * factor) as u32,
                            (scissor.height() * factor) as u32,
                        );
                    } else {
                        item_pass.set_scissor_rect(0, 0, size.width, size.height);
                    }

                    item_pass.draw_indexed(
                        draw.base_index..(draw.base_index + draw.vertex_count),
                        draw.vertex_offset,
                        draw.base_instance..(draw.base_instance + draw.instance_count),
                    );
                }
            }
        }

        {
            let mut antialiasing_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Antialiasing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.antialiasing_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            antialiasing_pass.set_pipeline(&self.gpu.antialiasing_resources.pipeline);
            antialiasing_pass.set_bind_group(
                0,
                self.gpu.gui_resources.antialiasing_bind_group(),
                &[],
            );
            antialiasing_pass.draw(0..3, 0..1);
        }

        {
            let mut combine_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Combine Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.second_combine_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            combine_pass.set_pipeline(&self.gpu.second_combine_resources.pipeline);
            combine_pass.set_bind_group(0, self.gpu.second_combine_resources.bind_group(), &[]);
            combine_pass.draw(0..3, 0..1)
        }

        {
            let view = output
                .texture
                .create_view(&TextureViewDescriptor::default());

            let mut present_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Present Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            present_pass.set_pipeline(&self.gpu.intermediate_resources.present_pipeline);
            present_pass.set_bind_group(
                0,
                self.gpu.intermediate_resources.present_bind_group(),
                &[],
            );
            present_pass.draw(0..3, 0..1)
        }

        fn size_align<T: PrimInt>(size: T, alignment: T) -> T {
            ((size + alignment - T::one()) / alignment) * alignment
        }

        let block_size = output.texture.format().block_size(None).unwrap();
        let texture_dim = output.texture.size();
        let buffer_dim = texture_dim.physical_size(output.texture.format());
        let padded_width = size_align(buffer_dim.width * block_size, COPY_BYTES_PER_ROW_ALIGNMENT);

        let screenshot_buffer = if setup.input_handler.key_active(KeyActions::Screenshot) {
            let intermediate_texture = self.gpu.device.create_texture(&TextureDescriptor {
                label: Some("Screenshot Intermediate Texture"),
                size: texture_dim,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SCREENSHOT_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            let intermediate_texture_view =
                intermediate_texture.create_view(&TextureViewDescriptor::default());

            let intermediate_bind_group = self.gpu.device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.gpu.intermediate_resources.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &self.gpu.second_combine_resources.texture().1,
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.gpu.non_filtering_sampler),
                    },
                ],
            });

            {
                let mut intermediate_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Screenshot Intermediate Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &intermediate_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                intermediate_pass
                    .set_pipeline(&self.gpu.intermediate_resources.screenshot_pipeline);
                intermediate_pass.set_bind_group(0, &intermediate_bind_group, &[]);
                intermediate_pass.draw(0..3, 0..1);
            }

            let buffer = self.gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Screenshot Buffer"),
                size: size_align(
                    (padded_width * buffer_dim.height) as BufferAddress,
                    COPY_BUFFER_ALIGNMENT,
                ),
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                intermediate_texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_width),
                        rows_per_image: Some(buffer_dim.height),
                    },
                },
                buffer_dim,
            );

            Some(buffer)
        } else {
            None
        };

        self.gpu
            .queue
            .submit(user_commands.into_iter().chain([encoder.finish()]));

        if let Some(buffer) = screenshot_buffer {
            {
                let slice = buffer.slice(..);

                let (tx, rx) = oneshot::channel();

                slice.map_async(MapMode::Read, move |result| {
                    tx.send(result).unwrap();
                });
                self.gpu.device.poll(Maintain::Wait);
                rx.blocking_recv().unwrap().unwrap();

                let texture_width = (texture_dim.width * block_size) as usize;
                let data = slice.get_mapped_range();
                let mut result = Vec::<u8>::new();
                for chunk in data.chunks_exact(padded_width as usize) {
                    for pixel in chunk[..texture_width].chunks_exact(4) {
                        result.extend(&[pixel[0], pixel[1], pixel[2], 255]);
                    }
                }

                if let Some(image) =
                    RgbaImage::from_vec(texture_dim.width, texture_dim.height, result)
                {
                    let mut clipboard = Clipboard::new().unwrap();

                    clipboard
                        .set_image(ImageData {
                            width: image.width() as usize,
                            height: image.height() as usize,
                            bytes: Cow::from(image.as_bytes()),
                        })
                        .unwrap();
                }
            }

            buffer.unmap();
        }

        output.present();

        Ok(())
    }
}
