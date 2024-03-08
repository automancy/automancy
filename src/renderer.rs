use std::borrow::Cow;
use std::f32::consts::FRAC_PI_6;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use arboard::{Clipboard, ImageData};
use egui::Rgba;
use egui_wgpu::wgpu::{
    BufferAddress, BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor,
    ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain, MapMode, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SurfaceError, TextureDescriptor, TextureDimension, TextureUsages, TextureViewDescriptor,
    COPY_BUFFER_ALIGNMENT, COPY_BYTES_PER_ROW_ALIGNMENT,
};
use egui_wgpu::ScreenDescriptor;
use hashbrown::HashMap;
use image::{EncodableLayout, RgbaImage};
use num::PrimInt;
use tokio::runtime::Runtime;
use tokio::sync::{oneshot, Mutex};
use wgpu::StoreOp;

use automancy_defs::coord::TileCoord;
use automancy_defs::glam::vec3;
use automancy_defs::gui::Gui;
use automancy_defs::id::Id;
use automancy_defs::math::{
    direction_to_angle, lerp_coords_to_pixel, Double, Float, Matrix4, FAR, HEX_GRID_LAYOUT, SQRT_3,
};
use automancy_defs::rendering::{make_line, GameUBO, InstanceData, LINE_DEPTH};
use automancy_defs::slice_group_by::GroupBy;
use automancy_defs::{bytemuck, colors};
use automancy_resources::data::Data;
use automancy_resources::ResourceManager;

use crate::game::{
    GameMsg, RenderInfo, RenderUnit, TransactionRecord, TransactionRecords,
    TRANSACTION_ANIMATION_SPEED,
};
use crate::gpu;
use crate::gpu::{
    AnimationMap, GlobalBuffers, Gpu, RenderResources, SharedResources, NORMAL_CLEAR,
    SCREENSHOT_FORMAT,
};
use crate::input::KeyActions;
use crate::options::Options;
use crate::setup::GameSetup;

pub struct Renderer<'a> {
    pub gpu: Gpu<'a>,
    pub shared_resources: SharedResources,
    pub render_resources: RenderResources,
    pub global_buffers: Arc<GlobalBuffers>,
    pub fps_limit: Double,

    render_info_cache: Arc<Mutex<Option<RenderInfo>>>,
    render_info_updating: Arc<AtomicBool>,
    transaction_records_cache: Arc<Mutex<TransactionRecords>>,
    transaction_records_updating: Arc<AtomicBool>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        gpu: Gpu<'a>,
        shared_resources: SharedResources,
        render_resources: RenderResources,
        global_buffers: Arc<GlobalBuffers>,
        options: &Options,
    ) -> Self {
        Self {
            gpu,
            shared_resources,
            render_resources,
            global_buffers,
            fps_limit: options.graphics.fps_limit,

            render_info_cache: Arc::new(Default::default()),
            render_info_updating: Arc::new(Default::default()),
            transaction_records_cache: Arc::new(Default::default()),
            transaction_records_updating: Arc::new(Default::default()),
        }
    }
}

fn get_angle_from_direction(target: &Data) -> Option<Float> {
    if let Data::Coord(target) = target {
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

pub fn try_add_animation(
    resource_man: &ResourceManager,
    start_instant: Instant,
    model: Id,
    animation_map: &mut AnimationMap,
) {
    if !animation_map.contains_key(&model) {
        let elapsed = Instant::now().duration_since(start_instant).as_secs_f32();

        let anims = resource_man.all_models[&model]
            .1
            .iter()
            .map(|anim| {
                let last = anim.inputs.last().unwrap();
                let wrapped = elapsed % last;
                let index = anim.inputs.partition_point(|v| *v < wrapped);

                (anim.target, anim.outputs[index])
            })
            .collect::<Vec<_>>();
        let anims = anims
            .binary_group_by_key(|v| v.0)
            .map(|v| (v[0].0, v.iter().fold(Matrix4::IDENTITY, |acc, v| acc * v.1)))
            .collect::<HashMap<_, _>>();

        animation_map.insert(model, anims);
    }
}

impl<'a> Renderer<'a> {
    pub fn render(
        &mut self,
        runtime: &Runtime,
        setup: &GameSetup,
        gui: &mut Gui,
        tile_tints: HashMap<TileCoord, Rgba>,
        mut extra_instances: Vec<(InstanceData, Id)>,
        mut in_world_item_instances: Vec<(InstanceData, Id)>,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();

        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let culling_range = setup.camera.culling_range;

        if !self.render_info_updating.load(Ordering::Relaxed) {
            let cache = self.render_info_cache.clone();
            let updating = self.render_info_updating.clone();
            let game = setup.game.clone();

            updating.store(true, Ordering::Relaxed);

            runtime.spawn(async move {
                let result = game
                    .call(
                        |reply| GameMsg::RenderInfoRequest {
                            culling_range,
                            reply,
                        },
                        None,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                *cache.lock().await = Some(result);

                updating.store(false, Ordering::Relaxed);
            });
        }

        let Some((mut instances, all_data)) = self.render_info_cache.blocking_lock().clone() else {
            return Ok(());
        };

        if !self.transaction_records_updating.load(Ordering::Relaxed) {
            let cache = self.transaction_records_cache.clone();
            let updating = self.transaction_records_updating.clone();
            let game = setup.game.clone();

            updating.store(true, Ordering::Relaxed);

            runtime.spawn(async move {
                let result = game
                    .call(GameMsg::GetRecordedTransactions, None)
                    .await
                    .unwrap()
                    .unwrap();

                *cache.lock().await = result;

                updating.store(false, Ordering::Relaxed);
            });
        }

        let camera_pos = setup.camera.get_pos();
        let camera_pos_float = camera_pos.as_vec3();
        let world_matrix = setup.camera.get_matrix().as_mat4();

        let mut animation_map = gui
            .renderer
            .callback_resources
            .insert(AnimationMap::new())
            .unwrap();

        let mut direction_previews = Vec::new();

        for (coord, unit) in instances.iter_mut() {
            let tile = setup
                .resource_man
                .registry
                .tiles
                .get(&unit.tile_id)
                .unwrap();

            if let Some(theta) = all_data
                .get(coord)
                .and_then(|data| data.get(&setup.resource_man.registry.data_ids.target))
                .and_then(get_angle_from_direction)
            {
                unit.instance = unit
                    .instance
                    .add_model_matrix(Matrix4::from_rotation_z(theta.to_radians()));

                if let Data::Color(color) = tile
                    .data
                    .get(&setup.resource_man.registry.data_ids.direction_color)
                    .unwrap_or(&Data::Color(colors::ORANGE))
                {
                    direction_previews.push((
                        InstanceData::default()
                            .with_color_offset(color.to_array())
                            .with_world_matrix(world_matrix)
                            .with_light_pos(camera_pos_float, None)
                            .with_model_matrix(
                                unit.instance.get_model_matrix()
                                    * Matrix4::from_rotation_z(FRAC_PI_6 * 5.0)
                                    * Matrix4::from_scale(vec3(0.1, SQRT_3, LINE_DEPTH))
                                    * Matrix4::from_translation(vec3(0.0, 0.5, 0.0)),
                            ),
                        setup.resource_man.registry.model_ids.cube1x1,
                        (),
                    ))
                }
            } else if let Some(Data::Id(inactive)) = tile
                .data
                .get(&setup.resource_man.registry.data_ids.inactive_model)
            {
                unit.model = setup.resource_man.get_model(*inactive);
            }
        }

        for (coord, data) in all_data {
            if let Some(Data::Coord(link)) = data.get(&setup.resource_man.registry.data_ids.link) {
                extra_instances.push((
                    InstanceData::default()
                        .with_color_offset(colors::RED.to_array())
                        .with_light_pos(camera_pos_float, None)
                        .with_world_matrix(world_matrix)
                        .with_model_matrix(make_line(
                            HEX_GRID_LAYOUT.hex_to_world_pos(*coord),
                            HEX_GRID_LAYOUT.hex_to_world_pos(**link),
                        )),
                    setup.resource_man.registry.model_ids.cube1x1,
                ));
            }
        }

        {
            let transaction_records = self.transaction_records_cache.blocking_lock();

            let now = Instant::now();

            for ((source_coord, coord), instants) in transaction_records.iter() {
                if culling_range.is_in_bounds(**source_coord) && culling_range.is_in_bounds(**coord)
                {
                    for (instant, TransactionRecord { stack, .. }) in instants {
                        let duration = now.duration_since(*instant);
                        let t = duration.as_secs_f64() / TRANSACTION_ANIMATION_SPEED.as_secs_f64();

                        let point = lerp_coords_to_pixel(*source_coord, *coord, t as Float);

                        let direction = *coord - *source_coord;
                        let direction = HEX_GRID_LAYOUT.hex_to_world_pos(*direction);
                        let theta = direction_to_angle(direction);

                        let instance = InstanceData::default()
                            .with_model_matrix(
                                Matrix4::from_translation(vec3(
                                    point.x as Float,
                                    point.y as Float,
                                    (FAR + 0.025) as Float,
                                )) * Matrix4::from_rotation_z(theta)
                                    * Matrix4::from_scale(vec3(0.3, 0.3, 0.3)),
                            )
                            .with_world_matrix(world_matrix)
                            .with_light_pos(camera_pos_float, None);
                        let model = setup.resource_man.get_item_model(stack.item);

                        in_world_item_instances.push((instance, model));
                    }
                }
            }
        }

        let mut game_instances = {
            let none = setup
                .resource_man
                .registry
                .tiles
                .get(&setup.resource_man.registry.none)
                .unwrap()
                .model;

            for hex in culling_range.all_coords() {
                let coord = TileCoord::from(hex);

                if !instances.contains_key(&coord) {
                    let p = HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

                    instances.insert(
                        coord,
                        RenderUnit {
                            instance: InstanceData::default().with_model_matrix(
                                Matrix4::from_translation(p.extend(FAR as Float)),
                            ),
                            tile_id: none,
                            model: none,
                        },
                    );
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
                try_add_animation(
                    &setup.resource_man,
                    setup.start_instant,
                    model,
                    &mut animation_map,
                );

                map.entry(model)
                    .or_insert_with(|| Vec::with_capacity(32))
                    .push((
                        instance
                            .with_light_pos(camera_pos_float, None)
                            .with_world_matrix(world_matrix),
                        model,
                        (),
                    ))
            }

            map.into_values().flatten().collect::<Vec<_>>()
        };

        for (_, model) in &extra_instances {
            try_add_animation(
                &setup.resource_man,
                setup.start_instant,
                *model,
                &mut animation_map,
            );
        }

        for (_, model) in &in_world_item_instances {
            try_add_animation(
                &setup.resource_man,
                setup.start_instant,
                *model,
                &mut animation_map,
            );
        }

        let mut extra_instances = extra_instances
            .into_iter()
            .map(|(instance, id)| (instance, id, ()))
            .collect::<Vec<_>>();
        game_instances.append(&mut extra_instances);

        let mut in_world_item_instances = in_world_item_instances
            .into_iter()
            .map(|(instance, id)| (instance, id, ()))
            .collect::<Vec<_>>();

        game_instances.append(&mut direction_previews);

        game_instances.sort_by_key(|v| v.1);
        in_world_item_instances.sort_by_key(|v| v.1);

        self.inner_render(
            setup,
            gui,
            &game_instances,
            &in_world_item_instances,
            &animation_map,
        )
    }

    fn inner_render(
        &mut self,
        setup: &GameSetup,
        gui: &mut Gui,
        game_instances: &[(InstanceData, Id, ())],
        in_world_item_instances: &[(InstanceData, Id, ())],
        animation_map: &AnimationMap,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();
        let factor = gui.context.pixels_per_point();

        let (game_instances, game_draws, game_draw_count, game_matrix_data) =
            gpu::indirect_instance(&setup.resource_man, game_instances, true, animation_map);

        let (
            in_world_item_instances,
            in_world_item_draws,
            in_world_item_draw_count,
            in_world_item_matrix_data,
        ) = gpu::indirect_instance(
            &setup.resource_man,
            in_world_item_instances,
            true,
            animation_map,
        );

        let egui_out = gui.context.end_frame();
        let egui_primitives = gui.context.tessellate(egui_out.shapes, factor);
        let egui_desc = ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: factor,
        };

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
                &mut self.render_resources.game_resources.instance_buffer,
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
                &mut self.render_resources.game_resources.indirect_buffer,
                indirect_buffer.as_slice(),
            );

            let mut game_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(NORMAL_CLEAR),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.model_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shared_resources.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if game_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.render_resources.game_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::default()]),
                );
                self.gpu.queue.write_buffer(
                    &self.render_resources.game_resources.matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(game_matrix_data.as_slice()),
                );

                game_pass.set_pipeline(&self.render_resources.game_resources.pipeline);
                game_pass.set_bind_group(0, &self.render_resources.game_resources.bind_group, &[]);
                game_pass.set_vertex_buffer(0, self.global_buffers.vertex_buffer.slice(..));
                game_pass.set_vertex_buffer(
                    1,
                    self.render_resources
                        .game_resources
                        .instance_buffer
                        .slice(..),
                );
                game_pass.set_index_buffer(
                    self.global_buffers.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                game_pass.multi_draw_indexed_indirect(
                    &self.render_resources.game_resources.indirect_buffer,
                    0,
                    game_draw_count,
                );
            }
        }

        {
            gpu::create_or_write_buffer(
                &self.gpu.device,
                &self.gpu.queue,
                &mut self
                    .render_resources
                    .in_world_item_resources
                    .instance_buffer,
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
                &mut self
                    .render_resources
                    .in_world_item_resources
                    .indirect_buffer,
                indirect_buffer.as_slice(),
            );

            let mut in_world_item_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("In-world Item Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.model_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shared_resources.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if in_world_item_draw_count > 0 {
                self.gpu.queue.write_buffer(
                    &self.render_resources.in_world_item_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::default()]),
                );
                self.gpu.queue.write_buffer(
                    &self
                        .render_resources
                        .in_world_item_resources
                        .matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(in_world_item_matrix_data.as_slice()),
                );

                in_world_item_pass
                    .set_pipeline(&self.render_resources.in_world_item_resources.pipeline);
                in_world_item_pass.set_bind_group(
                    0,
                    &self.render_resources.in_world_item_resources.bind_group,
                    &[],
                );
                in_world_item_pass
                    .set_vertex_buffer(0, self.global_buffers.vertex_buffer.slice(..));
                in_world_item_pass.set_vertex_buffer(
                    1,
                    self.render_resources
                        .in_world_item_resources
                        .instance_buffer
                        .slice(..),
                );
                in_world_item_pass.set_index_buffer(
                    self.global_buffers.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                in_world_item_pass.multi_draw_indexed_indirect(
                    &self
                        .render_resources
                        .in_world_item_resources
                        .indirect_buffer,
                    0,
                    in_world_item_draw_count,
                );
            }
        }

        {
            let mut post_processing_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Post Processing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self
                        .render_resources
                        .game_resources
                        .post_processing_texture()
                        .1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            post_processing_pass
                .set_pipeline(&self.render_resources.post_processing_resources.pipeline);
            post_processing_pass.set_bind_group(
                0,
                self.render_resources
                    .game_resources
                    .post_processing_bind_group(),
                &[],
            );
            post_processing_pass.draw(0..3, 0..1);
        }

        {
            let mut antialiasing_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Antialiasing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self
                        .render_resources
                        .game_resources
                        .antialiasing_texture()
                        .1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            antialiasing_pass
                .set_pipeline(&self.render_resources.antialiasing_resources.fxaa_pipeline);
            antialiasing_pass.set_bind_group(
                0,
                self.render_resources
                    .game_resources
                    .antialiasing_bind_group(),
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
                        view: &self.shared_resources.multisampling_texture().1,
                        resolve_target: Some(&self.render_resources.egui_resources.texture().1),
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &self.shared_resources.multisampling_depth_texture().1,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
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
                    view: &self.render_resources.first_combine_resources.texture().1,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            combine_pass.set_pipeline(&self.render_resources.first_combine_resources.pipeline);
            combine_pass.set_bind_group(
                0,
                self.render_resources.first_combine_resources.bind_group(),
                &[],
            );
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
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            present_pass.set_pipeline(
                &self
                    .render_resources
                    .intermediate_resources
                    .present_pipeline,
            );
            present_pass.set_bind_group(
                0,
                self.render_resources
                    .intermediate_resources
                    .present_bind_group(),
                &[],
            );
            present_pass.draw(0..3, 0..1)
        }

        fn size_align<T: PrimInt>(size: T, alignment: T) -> T {
            ((size + alignment - T::one()) / alignment) * alignment
        }

        let block_size = output.texture.format().block_copy_size(None).unwrap();
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

            {
                let mut intermediate_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Screenshot Intermediate Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &intermediate_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                intermediate_pass.set_pipeline(
                    &self
                        .render_resources
                        .intermediate_resources
                        .screenshot_pipeline,
                );
                intermediate_pass.set_bind_group(
                    0,
                    self.render_resources
                        .intermediate_resources
                        .present_bind_group(),
                    &[],
                );
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
