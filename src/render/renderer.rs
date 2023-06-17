use std::f32::consts::PI;
use std::sync::Arc;

use ractor::rpc::{multi_call, CallResult};
use tokio::runtime::Runtime;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage,
    RenderPassBeginInfo, SubpassContents,
};
use vulkano::format::ClearValue;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use vulkano::pipeline::graphics::viewport::Scissor;
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::swapchain::{acquire_next_image, AcquireError};
use vulkano::sync;
use vulkano::sync::GpuFuture;

use automancy_defs::cg::{deg, matrix, Float, Matrix4, Point3};
use automancy_defs::cgmath::{vec3, SquareMatrix};
use automancy_defs::colors;
use automancy_defs::colors::WithAlpha;
use automancy_defs::coord::{ChunkCoord, TileCoord};
use automancy_defs::egui_winit_vulkano::Gui;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::layout::hex_to_pixel;
use automancy_defs::id::Id;
use automancy_defs::rendering::{
    GameUBO, GameVertex, InstanceData, LightInfo, OverlayUBO, RawInstanceData, DEFAULT_LIGHT_COLOR,
    HEX_GRID_LAYOUT,
};
use automancy_resources::data::Data;
use automancy_resources::ResourceManager;

use crate::game::run::setup::GameSetup;
use crate::game::state::{GameMsg, RenderInfo, RenderUnit};
use crate::game::tile::entity::TileEntityMsg;
use crate::game::tile::ticking::TickUnit;
use crate::render::camera::FAR;
use crate::render::gpu;
use crate::render::gpu::Gpu;
use crate::util::actor::call_multi;

pub struct Renderer {
    resource_man: Arc<ResourceManager>,

    pub recreate_swapchain: bool,

    pub gpu: Gpu,

    tile_targets: Vec<(TileCoord, Option<Data>)>,
    last_tiles_update: Option<TickUnit>,
    previous_frame_end: Option<Box<dyn GpuFuture + Send + Sync>>,
}

impl Renderer {
    pub fn reset_last_tiles_update(&mut self) {
        self.last_tiles_update = None;
    }

    pub fn new(resource_man: Arc<ResourceManager>, gpu: Gpu) -> Self {
        let device = gpu.device.clone();

        Self {
            resource_man,

            recreate_swapchain: false,

            gpu,

            tile_targets: Default::default(),
            last_tiles_update: None,
            previous_frame_end: Some(sync::now(device).boxed_send_sync()),
        }
    }
}

impl Renderer {
    pub fn render(
        &mut self,
        runtime: &Runtime,
        setup: &GameSetup,
        map_render_info: &RenderInfo,
        gui_instances: Vec<(InstanceData, Id)>,
        extra_vertices: Vec<GameVertex>,
        gui: &mut Gui,
    ) {
        let update = {
            let new_last_tiles_update = runtime
                .block_on(setup.game.call(GameMsg::LastTilesUpdate, None))
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
                let coords = instances
                    .iter()
                    .flat_map(|(coord, RenderUnit { tile, .. })| {
                        if self
                            .resource_man
                            .registry
                            .tile(*tile)
                            .unwrap()
                            .model_attributes
                            .auto_rotate
                        {
                            Some(*coord)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let size = coords.len();
                if size > 0 {
                    let tile_entities = runtime
                        .block_on(call_multi(
                            &setup.game,
                            |reply| {
                                coords
                                    .into_iter()
                                    .map(|coord| GameMsg::GetTileEntityMulti(coord, reply.clone()))
                                    .collect::<Vec<_>>()
                            },
                            size,
                        ))
                        .unwrap();

                    let tile_entities = tile_entities.into_iter().flatten().collect::<Vec<_>>();

                    self.tile_targets = runtime
                        .block_on(multi_call(
                            tile_entities.as_slice(),
                            |reply| {
                                TileEntityMsg::GetDataValueAndCoord(
                                    setup.resource_man.registry.data_ids.target,
                                    reply,
                                )
                            },
                            None,
                        ))
                        .unwrap()
                        .into_iter()
                        .map(CallResult::unwrap)
                        .collect();
                }
            }

            for (coord, target) in &self.tile_targets {
                let theta: Float = if let Some(target) = target.as_ref().and_then(Data::as_coord) {
                    match *target {
                        TileCoord::RIGHT => -60.0,
                        TileCoord::BOTTOM_RIGHT => -120.0,
                        TileCoord::BOTTOM_LEFT => -180.0,
                        TileCoord::LEFT => -240.0,
                        TileCoord::TOP_LEFT => -300.0,
                        _ => 0.0,
                    }
                } else {
                    0.0
                };

                if let Some(m) = instances
                    .get_mut(coord)
                    .map(|v| &mut v.instance.model_matrix)
                {
                    *m = *m * Matrix4::from_angle_z(deg(theta))
                }
            }

            let a = [setup.camera_chunk_coord];
            let b = setup.camera_chunk_coord.neighbors();
            let c = setup.camera_chunk_coord.diagonals();

            let rendered_chunks = vec![a.as_slice(), b.as_slice(), c.as_slice()]
                .into_iter()
                .flatten()
                .flat_map(ChunkCoord::iter);

            for coord in rendered_chunks {
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

            if let Some(RenderUnit { instance, .. }) = instances.get_mut(&setup.camera.pointing_at)
            {
                *instance = instance.with_color_offset(colors::ORANGE.with_alpha(0.5).to_array())
            }

            let mut map = HashMap::new();

            for RenderUnit {
                instance, model, ..
            } in instances.into_values()
            {
                map.entry(model)
                    .or_insert_with(|| Vec::with_capacity(32))
                    .push((instance.into(), model))
            }

            map.into_values().flatten().collect::<Vec<_>>()
        };

        let (width, height) = gpu::window_size(&self.gpu.window);
        let aspect = width / height;
        let camera_pos = setup.camera.get_pos().cast::<Float>().unwrap();
        let matrix = matrix(camera_pos, aspect as Float, PI);

        self.inner_render(
            matrix,
            camera_pos,
            &instances,
            &gui_instances,
            extra_vertices,
            gui,
        );
    }

    fn inner_render(
        &mut self,
        matrix: Matrix4,
        camera_pos: Point3,
        instances: &[(RawInstanceData, Id)],
        gui_instances: &[(InstanceData, Id)],
        extra_vertices: Vec<GameVertex>,
        gui: &mut Gui,
    ) {
        let dimensions = gpu::window_size_u32(&self.gpu.window);

        if dimensions[0] == 0 || dimensions[1] == 0 {
            return;
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        self.gpu
            .resize_images(dimensions, &mut self.recreate_swapchain);

        if self.recreate_swapchain {
            self.gpu
                .recreate_swapchain(dimensions, &mut self.recreate_swapchain);
        }

        let (image_num, suboptimal, acquire_future) = {
            match acquire_next_image(self.gpu.alloc.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e:?}"),
            }
        };
        if suboptimal {
            self.recreate_swapchain = true;
        }
        let image_num = image_num as usize;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.gpu.alloc.command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        None,
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Depth(1.0)),
                        Some(ClearValue::Depth(1.0)),
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.gpu.framebuffers[image_num].clone())
                },
                SubpassContents::SecondaryCommandBuffers,
            )
            .unwrap();

        // game
        let mut game_builder = AutoCommandBufferBuilder::secondary(
            &self.gpu.alloc.command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.gpu.game_subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();

        if let Some((indirect_commands, instance_buffer)) =
            gpu::indirect_instance(&self.gpu.alloc.allocator, &self.resource_man, instances)
        {
            *self.gpu.alloc.game_uniform_buffer.write().unwrap() = GameUBO::new(matrix, camera_pos);

            game_builder
                .set_viewport(0, [gpu::viewport(&self.gpu.window)])
                .bind_pipeline_graphics(self.gpu.game_pipeline.clone())
                .bind_vertex_buffers(0, (self.gpu.alloc.vertex_buffer.clone(), instance_buffer))
                .bind_index_buffer(self.gpu.alloc.index_buffer.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.gpu.game_pipeline.layout().clone(),
                    0,
                    self.gpu.game_descriptor_set.clone(),
                )
                .draw_indexed_indirect(indirect_commands)
                .unwrap();
        }

        builder
            .execute_commands(game_builder.build().unwrap())
            .unwrap();

        builder
            .next_subpass(SubpassContents::SecondaryCommandBuffers)
            .unwrap();

        // extra gui
        let mut gui_builder = AutoCommandBufferBuilder::secondary(
            &self.gpu.alloc.command_allocator,
            self.gpu.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.gpu.gui_subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();

        {
            gui_builder
                .set_viewport(0, [gpu::viewport(&self.gpu.window)])
                .set_scissor(0, [Scissor::irrelevant()]);

            if !gui_instances.is_empty() {
                let instances = gui_instances
                    .iter()
                    .map(|(instance, id)| {
                        (
                            RawInstanceData::from(
                                instance.with_model_matrix(matrix * instance.model_matrix),
                            ),
                            *id,
                        )
                    })
                    .collect::<Vec<_>>();

                if let Some((indirect_commands, instance_buffer)) = gpu::indirect_instance(
                    &self.gpu.alloc.allocator,
                    &self.resource_man,
                    instances.as_slice(),
                ) {
                    let light_info = Buffer::from_data(
                        &self.gpu.alloc.allocator,
                        BufferCreateInfo {
                            usage: BufferUsage::VERTEX_BUFFER,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            usage: MemoryUsage::Upload,
                            ..Default::default()
                        },
                        LightInfo {
                            light_pos: camera_pos.into(),
                            light_color: DEFAULT_LIGHT_COLOR,
                        },
                    )
                    .unwrap();

                    gui_builder
                        .bind_pipeline_graphics(self.gpu.gui_pipeline.clone())
                        .bind_vertex_buffers(
                            0,
                            (
                                self.gpu.alloc.vertex_buffer.clone(),
                                instance_buffer,
                                light_info,
                            ),
                        )
                        .bind_index_buffer(self.gpu.alloc.index_buffer.clone())
                        .draw_indexed_indirect(indirect_commands)
                        .unwrap();
                }
            }

            if !extra_vertices.is_empty() {
                *self.gpu.alloc.overlay_uniform_buffer.write().unwrap() = OverlayUBO {
                    matrix: Matrix4::identity().into(),
                };

                let vertex_count = extra_vertices.len();

                let extra_vertex_buffer = Buffer::from_iter(
                    &self.gpu.alloc.allocator,
                    BufferCreateInfo {
                        usage: BufferUsage::VERTEX_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        usage: MemoryUsage::Upload,
                        ..Default::default()
                    },
                    extra_vertices.into_iter(),
                )
                .unwrap();

                gui_builder
                    .bind_pipeline_graphics(self.gpu.overlay_pipeline.clone())
                    .bind_vertex_buffers(0, extra_vertex_buffer)
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.gpu.overlay_pipeline.layout().clone(),
                        0,
                        self.gpu.overlay_descriptor_set.clone(),
                    )
                    .draw(vertex_count as u32, 1, 0, 0)
                    .unwrap();
            }
        }

        if let Ok(commands) = gui_builder.build() {
            builder.execute_commands(commands).unwrap();
        }

        // egui
        let egui_command_buffer = gui.draw_on_subpass_image(dimensions);
        builder.execute_commands(egui_command_buffer).unwrap();

        // end
        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();
        self.gpu.commit_commands(
            image_num,
            acquire_future,
            command_buffer,
            &mut self.previous_frame_end,
            &mut self.recreate_swapchain,
        );
    }
}
