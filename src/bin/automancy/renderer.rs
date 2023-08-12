use std::borrow::Cow;
use std::f32::consts::PI;
use std::time::Instant;

use arboard::{Clipboard, ImageData};
use egui::{Rect, Rgba};
use egui_wgpu::renderer::ScreenDescriptor;
use image::{EncodableLayout, RgbaImage};
use num::PrimInt;
use ractor::rpc::CallResult;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use wgpu::{
    BufferAddress, BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor,
    ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain, MapMode, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SurfaceError, TextureFormat, COPY_BUFFER_ALIGNMENT, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use automancy::game::{
    GameMsg, RenderInfo, RenderUnit, TickUnit, TransactionRecord, TRANSACTION_ANIMATION_SPEED,
};
use automancy::gpu;
use automancy::gpu::{Gpu, UPSCALE_LEVEL};
use automancy::input::KeyActions;
use automancy::tile_entity::TileEntityMsg;
use automancy::util::actor::multi_call_iter;
use automancy_defs::cgmath::{vec3, Angle, SquareMatrix};
use automancy_defs::coord::TileCoord;
use automancy_defs::gui::Gui;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::fractional::FractionalHex;
use automancy_defs::hexagon_tiles::traits::HexRound;
use automancy_defs::id::Id;
use automancy_defs::math::{deg, rad, z_far, z_near, DPoint3, Double, Float, Matrix4, Rad, FAR};
use automancy_defs::rendering::{
    make_line, GameUBO, InstanceData, OverlayUBO, PostEffectsUBO, RawInstanceData, Vertex,
};
use automancy_defs::{bytemuck, colors, math, window};
use automancy_resources::data::{Data, DataMap};

use crate::setup::GameSetup;

pub struct Renderer {
    pub gpu: Gpu,

    data_cache: HashMap<TileCoord, DataMap>,
    last_tiles_update: Option<TickUnit>,
}

impl Renderer {
    pub fn reset_last_tiles_update(&mut self) {
        self.last_tiles_update = None;
    }

    pub fn new(gpu: Gpu) -> Self {
        Self {
            gpu,

            data_cache: Default::default(),
            last_tiles_update: None,
        }
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

pub type GuiInstances = Vec<(
    InstanceData,
    Id,
    Option<Rect>,
    Option<Rect>,
    Option<(Float, Float)>,
)>;

impl Renderer {
    fn draw_links(
        coord: TileCoord,
        link: TileCoord,
        camera_pos: DPoint3,
        overlay: &mut Vec<Vertex>,
        (width, height): (Double, Double),
    ) {
        let (a, w0) = math::hex_to_normalized((width, height), camera_pos, coord);

        let (b, w1) = math::hex_to_normalized((width, height), camera_pos, link);

        overlay.extend_from_slice(&make_line(a, b, (w0 + w1) * 0.5, colors::RED));
    }

    pub fn render(
        &mut self,
        runtime: &Runtime,
        setup: &GameSetup,
        gui: &mut Gui,
        matrix: Matrix4,
        map_render_info: &RenderInfo,
        tile_tints: HashMap<TileCoord, Rgba>,
        gui_instances: GuiInstances,
        mut extra_instances: Vec<(RawInstanceData, Id)>,
        mut overlay: Vec<Vertex>,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();

        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        if let Some(size) = self.gpu.take_new_size() {
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            self.gpu.create_textures(size);
        }

        let culling_range = setup.camera.culling_range;
        let camera_coord = setup.camera.get_tile_coord();
        let camera_pos = setup.camera.get_pos();
        let camera_pos_float = camera_pos.cast::<Float>().unwrap();

        let instances = {
            let none = setup
                .resource_man
                .registry
                .tile(setup.resource_man.registry.none)
                .unwrap()
                .models[0];

            let mut instances = map_render_info.clone();

            let tile_entities = runtime
                .block_on(setup.game.call(
                    |reply| GameMsg::GetTileEntities {
                        center: camera_coord,
                        culling_range,
                        reply,
                    },
                    None,
                ))
                .unwrap()
                .unwrap(); // TODO just ask the game for everyone's data lmfao

            self.data_cache = runtime
                .block_on(multi_call_iter(
                    tile_entities.values(),
                    tile_entities.values().len(),
                    TileEntityMsg::GetDataWithCoord,
                    None,
                ))
                .unwrap()
                .into_iter()
                .map(CallResult::unwrap)
                .collect();

            for (coord, instance) in instances.iter_mut() {
                if let Some(theta) = self
                    .data_cache
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

                if let Some(link) = self
                    .data_cache
                    .get(coord)
                    .and_then(|data| data.get(&setup.resource_man.registry.data_ids.link))
                    .and_then(Data::as_coord)
                    .cloned()
                {
                    Self::draw_links(
                        *coord,
                        link,
                        camera_pos,
                        &mut overlay,
                        window::window_size_double(&self.gpu.window),
                    )
                }
            }

            let q0 = camera_coord.q() - culling_range.0 / 2;
            let q1 = camera_coord.q() + culling_range.0 / 2;

            let r0 = camera_coord.r() - culling_range.1 / 2;
            let r1 = camera_coord.r() + culling_range.1 / 2;

            for q in q0..q1 {
                for r in r0..r1 {
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
                    .push((
                        RawInstanceData::from(instance.with_light_pos(camera_pos_float)),
                        model,
                    ))
            }

            map.into_values().flatten().collect::<Vec<_>>()
        };

        let transaction_records = runtime
            .block_on(setup.game.call(GameMsg::GetRecordedTransactions, None))
            .unwrap()
            .unwrap();
        let now = Instant::now();

        let transaction_records_read = transaction_records.read().unwrap();

        for ((source_coord, coord), instants) in transaction_records_read.iter() {
            for (instant, TransactionRecord { stack, .. }) in instants {
                let duration = now.duration_since(*instant);
                let t = duration.as_secs_f64() / TRANSACTION_ANIMATION_SPEED.as_secs_f64();
                let a = FractionalHex::new(source_coord.q() as Double, source_coord.r() as Double);
                let b = FractionalHex::new(coord.q() as Double, coord.r() as Double);
                let lerp = a.lerp(b, t);
                let point = math::frac_hex_to_pixel(lerp);

                let direction = *coord - *source_coord;
                let direction = math::hex_to_pixel(direction.into());
                let angle = Rad::atan2(direction.y as Float, direction.x as Float);
                let angle = rad(angle.0.rem_euclid(PI));

                let instance = InstanceData::default()
                    .with_model_matrix(
                        Matrix4::from_translation(vec3(
                            point.x as Float,
                            point.y as Float,
                            FAR as Float,
                        )) * Matrix4::from_scale(0.3)
                            * Matrix4::from_angle_z(angle),
                    )
                    .with_light_pos(camera_pos_float);
                let model = setup.resource_man.get_item_model(stack.item);

                extra_instances.push((instance.into(), model));
            }
        }

        extra_instances.sort_by_key(|v| v.1);

        self.inner_render(
            setup,
            gui,
            matrix,
            &instances,
            &extra_instances,
            gui_instances,
            overlay,
        )
    }

    fn inner_render(
        &mut self,
        setup: &GameSetup,
        gui: &mut Gui,
        matrix: Matrix4,
        instances: &[(RawInstanceData, Id)],
        extra_instances: &[(RawInstanceData, Id)],
        gui_instances: GuiInstances,
        overlay: Vec<Vertex>,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();
        let output = self.gpu.surface.get_current_texture()?;

        {
            let output_size = output.texture.size();

            if output_size.width != size.width || output_size.height != size.height {
                return Ok(());
            }
        }

        let factor = gui.context.pixels_per_point();

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let count = gpu::indirect_instance(
                &self.gpu.device,
                &self.gpu.queue,
                &setup.resource_man,
                instances,
                &mut self.gpu.game_resources.game_instance_buffer,
                &mut self.gpu.game_resources.game_indirect_buffer,
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
                        view: &self.gpu.position_texture().1,
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
                            load: LoadOp::Clear(Color::TRANSPARENT),
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

            if count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.game_resources.game_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(matrix)]),
                );

                game_pass.set_viewport(
                    0.0,
                    0.0,
                    (size.width * UPSCALE_LEVEL) as Float,
                    (size.height * UPSCALE_LEVEL) as Float,
                    1.0,
                    0.0,
                );
                game_pass.set_pipeline(&self.gpu.game_resources.game_pipeline);
                game_pass.set_bind_group(0, &self.gpu.game_resources.game_bind_group, &[]);
                game_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                game_pass
                    .set_vertex_buffer(1, self.gpu.game_resources.game_instance_buffer.slice(..));
                game_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                game_pass.multi_draw_indexed_indirect(
                    &self.gpu.game_resources.game_indirect_buffer,
                    0,
                    count,
                );
            }
        }

        {
            let count = gpu::indirect_instance(
                &self.gpu.device,
                &self.gpu.queue,
                &setup.resource_man,
                extra_instances,
                &mut self.gpu.extra_resources.extra_instance_buffer,
                &mut self.gpu.extra_resources.extra_indirect_buffer,
            );

            let mut extra_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Extra Render Pass"),
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
                        view: &self.gpu.position_texture().1,
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
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.extra_resources.extra_depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if count > 0 {
                self.gpu.queue.write_buffer(
                    &self.gpu.extra_resources.extra_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(matrix)]),
                );

                extra_pass.set_viewport(
                    0.0,
                    0.0,
                    (size.width * UPSCALE_LEVEL) as Float,
                    (size.height * UPSCALE_LEVEL) as Float,
                    1.0,
                    0.0,
                );
                extra_pass.set_pipeline(&self.gpu.game_resources.game_pipeline);
                extra_pass.set_bind_group(0, &self.gpu.game_resources.game_bind_group, &[]);
                extra_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                extra_pass
                    .set_vertex_buffer(1, self.gpu.extra_resources.extra_instance_buffer.slice(..));
                extra_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                extra_pass.multi_draw_indexed_indirect(
                    &self.gpu.extra_resources.extra_indirect_buffer,
                    0,
                    count,
                );
            }
        }

        {
            let mut effects_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Effects Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.effects_resources.processed_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            effects_pass.set_pipeline(&self.gpu.effects_resources.effects_pipeline);
            effects_pass.set_bind_group(0, self.gpu.effects_resources.effects_bind_group(), &[]);

            effects_pass.draw(0..3, 0..1);
        }

        {
            let mut post_effects_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Post Effects Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.post_effects_resources.processed_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.gpu.queue.write_buffer(
                &self.gpu.game_resources.game_post_effects_uniform_buffer,
                0,
                bytemuck::cast_slice(&[PostEffectsUBO {
                    z_near: z_near(),
                    z_far: z_far(),
                }]),
            );

            post_effects_pass.set_pipeline(&self.gpu.post_effects_resources.post_effects_pipeline);

            post_effects_pass.set_bind_group(
                0,
                self.gpu.game_resources.game_post_effects_bind_group(),
                &[],
            );

            post_effects_pass.draw(0..3, 0..1);
        }

        {
            let mut overlay_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Overlay Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.post_effects_resources.processed_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            if !overlay.is_empty() {
                self.gpu.queue.write_buffer(
                    &self.gpu.overlay_resources.overlay_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[OverlayUBO::new(Matrix4::identity())]),
                );
                gpu::create_or_write_buffer(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut self.gpu.overlay_resources.overlay_vertex_buffer,
                    bytemuck::cast_slice(overlay.as_slice()),
                );

                let vertex_count = overlay.len() as u32;

                overlay_pass.set_pipeline(&self.gpu.overlay_resources.overlay_pipeline);
                overlay_pass.set_bind_group(0, &self.gpu.overlay_resources.overlay_bind_group, &[]);
                overlay_pass.set_vertex_buffer(
                    0,
                    self.gpu.overlay_resources.overlay_vertex_buffer.slice(..),
                );

                overlay_pass.draw(0..vertex_count, 0..1);
            }
        }

        let user_commands = {
            let egui_out = gui.context.end_frame();
            let egui_primitives = gui.context.tessellate(egui_out.shapes);
            let egui_desc = ScreenDescriptor {
                size_in_pixels: [size.width, size.height],
                pixels_per_point: factor,
            };

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
                        view: &self.gpu.egui_resources.egui_texture().1,
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
                    view: &self.gpu.first_combine_resources.combine_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            combine_pass.set_pipeline(&self.gpu.first_combine_resources.combine_pipeline);
            combine_pass.set_bind_group(
                0,
                self.gpu.first_combine_resources.combine_bind_group(),
                &[],
            );

            combine_pass.draw(0..3, 0..1)
        }

        {
            let (instances, draws): (Vec<_>, Vec<_>) = gui_instances
                .into_iter()
                .map(|(instance, id, viewport, scissor, depth)| {
                    (
                        RawInstanceData::from(instance),
                        (id, viewport, scissor, depth),
                    )
                })
                .unzip();

            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.gui_resources.gui_instance_buffer,
                bytemuck::cast_slice(instances.as_slice()),
            );

            let mut gui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.game_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if !draws.is_empty() {
                self.gpu.queue.write_buffer(
                    &self.gpu.gui_resources.gui_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::default()]),
                );

                gui_pass.set_pipeline(&self.gpu.gui_resources.gui_pipeline);
                gui_pass.set_bind_group(0, &self.gpu.gui_resources.gui_bind_group, &[]);
                gui_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                gui_pass.set_vertex_buffer(1, self.gpu.gui_resources.gui_instance_buffer.slice(..));
                gui_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                for (idx, (id, viewport, scissor, depth)) in draws.into_iter().enumerate() {
                    let idx = idx as u32;

                    let depth = depth.unwrap_or((1.0, 0.0));

                    if let Some(viewport) = viewport {
                        gui_pass.set_viewport(
                            viewport.left() * factor * UPSCALE_LEVEL as Float,
                            viewport.top() * factor * UPSCALE_LEVEL as Float,
                            viewport.width() * factor * UPSCALE_LEVEL as Float,
                            viewport.height() * factor * UPSCALE_LEVEL as Float,
                            depth.0,
                            depth.1,
                        );
                    } else {
                        gui_pass.set_viewport(
                            0.0,
                            0.0,
                            (size.width * UPSCALE_LEVEL) as Float,
                            (size.height * UPSCALE_LEVEL) as Float,
                            depth.0,
                            depth.1,
                        );
                    }

                    if let Some(scissor) = scissor {
                        gui_pass.set_scissor_rect(
                            (scissor.left() * factor) as u32 * UPSCALE_LEVEL,
                            (scissor.top() * factor) as u32 * UPSCALE_LEVEL,
                            (scissor.width() * factor) as u32 * UPSCALE_LEVEL,
                            (scissor.height() * factor) as u32 * UPSCALE_LEVEL,
                        );
                    } else {
                        gui_pass.set_scissor_rect(
                            0,
                            0,
                            size.width * UPSCALE_LEVEL,
                            size.height * UPSCALE_LEVEL,
                        );
                    }

                    let index_range = setup.resource_man.index_ranges[&id];

                    let a = index_range.offset;
                    let b = a + index_range.size;
                    gui_pass.draw_indexed(a..b, 0, idx..(idx + 1));
                }
            }
        }

        {
            let mut effects_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Effects Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.effects_resources.processed_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            effects_pass.set_pipeline(&self.gpu.effects_resources.effects_pipeline);
            effects_pass.set_bind_group(0, self.gpu.effects_resources.effects_bind_group(), &[]);

            effects_pass.draw(0..3, 0..1);
        }

        {
            let mut combine_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Combine Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.second_combine_resources.combine_texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            combine_pass.set_pipeline(&self.gpu.second_combine_resources.combine_pipeline);
            combine_pass.set_bind_group(
                0,
                self.gpu.second_combine_resources.combine_bind_group(),
                &[],
            );

            combine_pass.draw(0..3, 0..1)
        }

        let view = output.texture.as_image_copy();

        encoder.copy_texture_to_texture(
            self.gpu
                .second_combine_resources
                .combine_texture()
                .0
                .as_image_copy(),
            view,
            output.texture.size(),
        );

        fn size_align<T: PrimInt>(size: T, alignment: T) -> T {
            ((size + alignment - T::one()) / alignment) * alignment
        }

        let block_size = output.texture.format().block_size(None).unwrap();
        let texture_dim = output.texture.size();
        let buffer_dim = texture_dim.physical_size(output.texture.format());
        let padded_width = size_align(buffer_dim.width * block_size, COPY_BYTES_PER_ROW_ALIGNMENT);

        let screenshot_buffer = if setup.input_handler.key_active(KeyActions::Screenshot) {
            let buffer = self.gpu.device.create_buffer(&BufferDescriptor {
                label: None,
                size: size_align(
                    (padded_width * buffer_dim.height) as BufferAddress,
                    COPY_BUFFER_ALIGNMENT,
                ),
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                output.texture.as_image_copy(),
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

                // TODO does screenshotting work on windows

                let texture_width = texture_dim.width * block_size;

                let data = slice.get_mapped_range();
                let mut result = Vec::new();
                for chunk in data.chunks(padded_width as usize) {
                    match output.texture.format().remove_srgb_suffix() {
                        TextureFormat::Rgba8Unorm => {
                            result.extend(&chunk[..texture_width as usize]);
                        }
                        TextureFormat::Bgra8Unorm => {
                            result.extend(
                                chunk[..texture_width as usize]
                                    .chunks_exact(4)
                                    .flat_map(|v| [v[2], v[1], v[0], v[3]]),
                            );
                        }
                        _ => {}
                    }
                }

                if let Some(image) =
                    RgbaImage::from_raw(texture_dim.width, texture_dim.height, result)
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
