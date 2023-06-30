use std::sync::Arc;
use std::time::Instant;

use egui::{pos2, Rect, Rgba};
use egui_wgpu::renderer::ScreenDescriptor;
use ractor::rpc::CallResult;
use ractor::ActorRef;
use tokio::runtime::Runtime;
use wgpu::{
    Color, CommandEncoderDescriptor, IndexFormat, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, SurfaceError, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use automancy_defs::bytemuck;
use automancy_defs::cg::{deg, Double, Float, Matrix4, Point3};
use automancy_defs::cgmath::{vec3, SquareMatrix};
use automancy_defs::coord::{TileCoord, TileUnit};
use automancy_defs::gui::Gui;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::fractional::FractionalHex;
use automancy_defs::hexagon_tiles::layout::{frac_hex_to_pixel, hex_to_pixel};
use automancy_defs::hexagon_tiles::traits::HexRound;
use automancy_defs::id::Id;
use automancy_defs::rendering::{
    GameUBO, InstanceData, OverlayUBO, RawInstanceData, Vertex, HEX_GRID_LAYOUT,
};
use automancy_resources::data::Data;
use automancy_resources::ResourceManager;

use crate::camera::FAR;
use crate::game::{GameMsg, RenderInfo, RenderUnit, TickUnit, ANIMATION_SPEED};
use crate::gpu;
use crate::gpu::{Gpu, GUI_INSTANCE_BUFFER, OVERLAY_VERTEX_BUFFER};
use crate::tile_entity::TileEntityMsg;
use crate::util::actor::multi_call_iter;

pub struct Renderer {
    pub resized: bool,
    pub size: PhysicalSize<u32>,

    pub gpu: Gpu,
    resource_man: Arc<ResourceManager>,

    tile_targets: HashMap<TileCoord, Data>,
    last_tiles_update: Option<TickUnit>,
}

impl Renderer {
    pub fn reset_last_tiles_update(&mut self) {
        self.last_tiles_update = None;
    }

    pub fn new(resource_man: Arc<ResourceManager>, gpu: Gpu) -> Self {
        Self {
            resized: false,

            size: gpu.window.inner_size(),
            gpu,
            resource_man,

            tile_targets: Default::default(),
            last_tiles_update: None,
        }
    }
}

fn get_angle_from_target(target: Option<&Data>) -> Option<Float> {
    if let Some(target) = target.and_then(Data::as_coord) {
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

pub type GuiInstances = Vec<(InstanceData, Id, Rect, (Float, Float))>;

impl Renderer {
    pub fn render(
        &mut self,
        runtime: &Runtime,
        resource_man: Arc<ResourceManager>,
        camera_pos: Point3,
        camera_coord: TileCoord,
        matrix: Matrix4,
        culling_range: (TileUnit, TileUnit),
        game: ActorRef<GameMsg>,
        map_render_info: &RenderInfo,
        tile_tints: HashMap<TileCoord, Rgba>,
        mut gui_instances: GuiInstances,
        overlay: Vec<Vertex>,
        gui: &mut Gui,
    ) -> Result<(), SurfaceError> {
        let update = {
            let new_last_tiles_update = runtime
                .block_on(game.call(GameMsg::LastTilesUpdate, None))
                .unwrap()
                .unwrap();

            if self.last_tiles_update.is_some() {
                if self.last_tiles_update.unwrap() < new_last_tiles_update {
                    self.last_tiles_update = Some(new_last_tiles_update);
                    true
                } else {
                    false
                }
            } else {
                self.last_tiles_update = Some(new_last_tiles_update);
                true
            }
        };

        let instances = {
            let none = self
                .resource_man
                .registry
                .tile(self.resource_man.registry.none)
                .unwrap()
                .models[0];

            let mut instances = map_render_info.clone();

            if update {
                let tile_entities = runtime
                    .block_on(game.call(
                        |reply| GameMsg::GetTileEntities {
                            center: camera_coord,
                            culling_range,
                            reply,
                        },
                        None,
                    ))
                    .unwrap()
                    .unwrap();

                self.tile_targets = runtime
                    .block_on(multi_call_iter(
                        tile_entities.values(),
                        tile_entities.values().len(),
                        |reply| {
                            TileEntityMsg::GetDataValueWithCoord(
                                resource_man.registry.data_ids.target,
                                reply,
                            )
                        },
                        None,
                    ))
                    .unwrap()
                    .into_iter()
                    .map(CallResult::unwrap)
                    .flat_map(|(a, b)| Some(a).zip(b))
                    .collect();
            }

            for (coord, target) in &self.tile_targets {
                if let Some(instance) = instances.get_mut(coord) {
                    let theta = get_angle_from_target(Some(target));

                    if let Some(theta) = theta {
                        let m = &mut instance.instance.model_matrix;

                        *m = *m * Matrix4::from_angle_z(deg(theta))
                    } else if let Some(inactive) = self
                        .resource_man
                        .registry
                        .tile(instance.tile)
                        .unwrap()
                        .model_attributes
                        .inactive_model
                    {
                        instance.model = inactive;
                    }
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
                        let p = hex_to_pixel(HEX_GRID_LAYOUT, coord.into());

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
                    .push((RawInstanceData::from(instance), model))
            }

            map.into_values().flatten().collect::<Vec<_>>()
        };

        let transaction_record = runtime
            .block_on(game.call(GameMsg::GetRecordedTransactions, None))
            .unwrap()
            .unwrap();
        let now = Instant::now();

        let (width, height) = gpu::window_size_float(&self.gpu.window);

        for (instant, ((source_coord, _source_id), (coord, _id)), stack) in
            transaction_record.read().unwrap().iter()
        {
            let duration = now.duration_since(*instant);
            let t = duration.as_secs_f64() / ANIMATION_SPEED.as_secs_f64();
            let a = FractionalHex::new(source_coord.q() as Double, source_coord.r() as Double);
            let b = FractionalHex::new(coord.q() as Double, coord.r() as Double);
            let lerp = a.lerp(b, t);
            let point = frac_hex_to_pixel(HEX_GRID_LAYOUT, lerp);

            let instance = InstanceData::default().with_model_matrix(
                matrix
                    * Matrix4::from_translation(vec3(
                        point.x as Float,
                        point.y as Float,
                        FAR as Float,
                    ))
                    * Matrix4::from_scale(0.5)
                    * Matrix4::from_angle_z(deg(get_angle_from_target(
                        self.tile_targets.get(source_coord),
                    )
                    .map(|v| v + 60.0)
                    .unwrap_or(0.0))),
            );
            let id = resource_man.get_item_model(stack.item);

            gui_instances.push((
                instance,
                id,
                Rect::from([pos2(0.0, 0.0), pos2(width, height)]),
                (0.0, 1.0),
            ));
        }

        self.inner_render(
            &resource_man,
            matrix,
            camera_pos,
            &instances,
            gui_instances,
            overlay,
            gui,
        )
    }

    fn inner_render(
        &mut self,
        resource_man: &ResourceManager,
        matrix: Matrix4,
        camera_pos: Point3,
        instances: &[(RawInstanceData, Id)],
        gui_instances: GuiInstances,
        overlay: Vec<Vertex>,
        gui: &mut Gui,
    ) -> Result<(), SurfaceError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }

        if self.resized {
            self.gpu.resize(self.size);
            self.resized = false;
        }

        let output = self.gpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut game_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.multisample_surface_texture.1,
                    resolve_target: Some(&view),
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.game_depth_texture.1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            self.gpu.queue.write_buffer(
                &self.gpu.game_uniform_buffer,
                0,
                bytemuck::cast_slice(&[GameUBO::new(matrix, camera_pos)]),
            );

            let count = gpu::indirect_instance(
                &self.gpu.device,
                &self.gpu.queue,
                &self.resource_man,
                instances,
                &mut self.gpu.game_instance_buffer,
                &mut self.gpu.game_indirect_buffer,
            );

            if count > 0 {
                game_pass.set_viewport(
                    0.0,
                    0.0,
                    self.size.width as Float,
                    self.size.height as Float,
                    1.0,
                    0.0,
                );
                game_pass.set_pipeline(&self.gpu.game_pipeline);
                game_pass.set_bind_group(0, &self.gpu.game_bind_group, &[]);
                game_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
                game_pass.set_vertex_buffer(1, self.gpu.game_instance_buffer.slice(..));
                game_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

                game_pass.multi_draw_indexed_indirect(&self.gpu.game_indirect_buffer, 0, count);
            }
        }

        let user_commands = {
            let egui_out = gui.context.end_frame();
            let egui_primitives = gui.context.tessellate(egui_out.shapes);
            let egui_desc = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point: gui.context.pixels_per_point(),
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
                        view: &self.gpu.multisample_surface_texture.1,
                        resolve_target: Some(&view),
                        ops: Operations {
                            load: LoadOp::Load,
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

        if !gui_instances.is_empty() {
            let mut gui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.multisample_surface_texture.1,
                    resolve_target: Some(&view),
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.gpu.gui_depth_texture.1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            let (instances, draws): (Vec<_>, Vec<_>) = gui_instances
                .into_iter()
                .map(|(instance, id, rect, depth)| {
                    (RawInstanceData::from(instance), (id, rect, depth))
                })
                .unzip();

            self.gpu.queue.write_buffer(
                &self.gpu.gui_uniform_buffer,
                0,
                bytemuck::cast_slice(&[GameUBO::default()]),
            );

            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.gui_instance_buffer,
                GUI_INSTANCE_BUFFER,
                bytemuck::cast_slice(instances.as_slice()),
            );

            gui_pass.set_pipeline(&self.gpu.gui_pipeline);
            gui_pass.set_bind_group(0, &self.gpu.gui_bind_group, &[]);
            gui_pass.set_vertex_buffer(0, self.gpu.vertex_buffer.slice(..));
            gui_pass.set_vertex_buffer(1, self.gpu.gui_instance_buffer.slice(..));
            gui_pass.set_index_buffer(self.gpu.index_buffer.slice(..), IndexFormat::Uint16);

            for (idx, (id, rect, depth)) in draws.into_iter().enumerate() {
                let idx = idx as u32;

                gui_pass.set_viewport(
                    rect.left(),
                    rect.top(),
                    rect.width(),
                    rect.height(),
                    depth.0,
                    depth.1,
                );

                let index_range = resource_man.index_ranges.get(&id).unwrap_or_else(|| {
                    &resource_man.index_ranges[&resource_man.registry.model_ids.missing]
                });
                let a = index_range.offset;
                let b = a + index_range.size;
                gui_pass.draw_indexed(a..b, 0, idx..(idx + 1));
            }
        }

        if !overlay.is_empty() {
            let mut overlay_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Overlay Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.gpu.multisample_surface_texture.1,
                    resolve_target: Some(&view),
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.gpu.queue.write_buffer(
                &self.gpu.overlay_uniform_buffer,
                0,
                bytemuck::cast_slice(&[OverlayUBO::new(Matrix4::identity())]),
            );
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self.gpu.overlay_vertex_buffer,
                OVERLAY_VERTEX_BUFFER,
                bytemuck::cast_slice(overlay.as_slice()),
            );

            let vertex_count = overlay.len() as u32;

            overlay_pass.set_pipeline(&self.gpu.overlay_pipeline);
            overlay_pass.set_bind_group(0, &self.gpu.overlay_bind_group, &[]);
            overlay_pass.set_vertex_buffer(0, self.gpu.overlay_vertex_buffer.slice(..));

            overlay_pass.draw(0..vertex_count, 0..1)
        }

        self.gpu
            .queue
            .submit(user_commands.into_iter().chain([encoder.finish()]));
        output.present();

        Ok(())
    }
}
