use std::borrow::Cow;
use std::collections::VecDeque;
use std::f32::consts::FRAC_PI_6;
use std::mem;
use std::sync::Arc;
use std::time::Instant;

use arboard::{Clipboard, ImageData};
use hashbrown::HashMap;
use image::{EncodableLayout, RgbaImage};
use tokio::sync::oneshot;
use wgpu::{
    BufferAddress, BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor,
    ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain, MapMode, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SurfaceError, TextureDescriptor, TextureDimension, TextureUsages, TextureViewDescriptor,
    COPY_BUFFER_ALIGNMENT, COPY_BYTES_PER_ROW_ALIGNMENT,
};
use wgpu::{CommandBuffer, StoreOp};

use automancy_defs::slice_group_by::GroupBy;
use automancy_defs::{colors, math};
use automancy_defs::{coord::TileCoord, math::Vec4};
use automancy_defs::{
    glam::vec2,
    rendering::{make_line, GameUBO, InstanceData, LINE_DEPTH},
};
use automancy_defs::{glam::vec3, rendering::PostProcessingUBO};
use automancy_defs::{id::Id, math::get_screen_world_bounding_vec};
use automancy_defs::{
    math::{
        direction_to_angle, lerp_coords_to_pixel, Float, Matrix4, FAR, HEX_GRID_LAYOUT, SQRT_3,
    },
    window,
};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;
use yakui::Rect;
use yakui_wgpu::SurfaceInfo;

use crate::gpu::{
    AnimationMap, GlobalResources, Gpu, RenderResources, SharedResources, MODEL_DEPTH_CLEAR,
    NORMAL_CLEAR, SCREENSHOT_FORMAT,
};
use crate::{
    game::{GameSystemMessage, TransactionRecord, TransactionRecords, TRANSACTION_ANIMATION_SPEED},
    gui::GameElementPaint,
};
use crate::{
    game::{RenderUnit, RenderUnits},
    gpu::IndirectInstanceDrawData,
    gui::YakuiRenderResources,
    GameState,
};
use crate::{gpu, gui};

type RenderInfo = (RenderUnits, HashMap<TileCoord, DataMap>);

pub struct Renderer {
    pub gpu: Gpu,
    pub shared_resources: SharedResources,
    pub render_resources: RenderResources,
    pub global_resources: Arc<GlobalResources>,

    render_info_buffer: [Option<RenderInfo>; 2],
    render_info_cursor: usize,
    transaction_records_buffer: [Option<TransactionRecords>; 2],
    transaction_records_cursor: usize,

    pub tile_tints: HashMap<TileCoord, Vec4>,
    pub extra_instances: Vec<(InstanceData, Id, ())>,
    pub overlay_instances: Vec<(InstanceData, Id, ())>,

    pub take_item_animations: HashMap<Id, VecDeque<(Instant, Rect)>>,

    screenshot_clipboard: Clipboard,
}

impl Renderer {
    pub fn new(
        gpu: Gpu,
        shared_resources: SharedResources,
        render_resources: RenderResources,
        global_resources: Arc<GlobalResources>,
    ) -> Self {
        Self {
            gpu,
            shared_resources,
            render_resources,
            global_resources,

            render_info_buffer: Default::default(),
            render_info_cursor: 0,
            transaction_records_buffer: Default::default(),
            transaction_records_cursor: 0,

            tile_tints: Default::default(),
            extra_instances: vec![],
            overlay_instances: vec![],

            take_item_animations: Default::default(),

            screenshot_clipboard: Clipboard::new().unwrap(),
        }
    }
}

pub fn try_add_animation(
    resource_man: &ResourceManager,
    start_instant: Instant,
    model: Id,
    animation_map: &mut AnimationMap,
) -> bool {
    if !animation_map.contains_key(&model) {
        let elapsed = Instant::now().duration_since(start_instant).as_secs_f32();

        if let Some((_, anims)) = resource_man.all_models.get(&model) {
            let anims = anims
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

            return true;
        } else {
            return false;
        }
    }

    true
}

impl Renderer {
    pub fn render(state: &mut GameState, screenshotting: bool) -> Result<(), SurfaceError> {
        let Some(renderer) = state.renderer.as_mut() else {
            return Ok(());
        };

        let tile_tints = mem::take(&mut renderer.tile_tints);
        let mut extra_instances = mem::take(&mut renderer.extra_instances);
        let mut overlay_instances = mem::take(&mut renderer.overlay_instances);

        let size = renderer.gpu.window.inner_size();

        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let mut animation_map = AnimationMap::new();
        let camera_pos = state.camera.get_pos();
        let camera_pos_float = camera_pos.as_vec3();
        let culling_range = state.camera.culling_range;

        let Some((render_info, all_data)) = ({
            let len = renderer.render_info_buffer.len();
            let cursor = renderer.render_info_cursor;
            let next = (cursor + 1) % len;
            assert_ne!(cursor, next);

            let ptr = renderer.render_info_buffer.as_mut_ptr();
            let cursor = unsafe { ptr.wrapping_add(cursor).as_ref().unwrap() };
            let next = unsafe { ptr.wrapping_add(next).as_mut().unwrap() };

            let game = state.game.clone();
            state.tokio.spawn(async move {
                let all_data = game
                    .call(GameSystemMessage::GetAllData, None)
                    .await
                    .unwrap()
                    .unwrap();
                let instances = game
                    .call(
                        |reply| GameSystemMessage::GetAllRenderUnits {
                            reply,
                            culling_range,
                        },
                        None,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                *next = Some((instances, all_data));
            });

            renderer.render_info_cursor = (renderer.render_info_cursor + 1) % len;

            cursor
        }) else {
            return Ok(());
        };

        let Some(transaction_records) = ({
            let len = renderer.transaction_records_buffer.len();
            let cursor = renderer.transaction_records_cursor;
            let next = (cursor + 1) % len;
            assert_ne!(cursor, next);

            let ptr = renderer.transaction_records_buffer.as_mut_ptr();
            let cursor = unsafe { ptr.wrapping_add(cursor).as_ref().unwrap() };
            let next = unsafe { ptr.wrapping_add(next).as_mut().unwrap() };

            let game = state.game.clone();
            state.tokio.spawn(async move {
                let result = game
                    .call(GameSystemMessage::GetRecordedTransactions, None)
                    .await
                    .unwrap()
                    .unwrap();

                *next = Some(result);
            });

            renderer.transaction_records_cursor = (renderer.transaction_records_cursor + 1) % len;

            cursor
        }) else {
            return Ok(());
        };

        {
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

                        let instance = InstanceData::default().with_model_matrix(
                            Matrix4::from_translation(vec3(
                                point.x as Float,
                                point.y as Float,
                                (FAR + 0.025) as Float,
                            )) * Matrix4::from_rotation_z(theta)
                                * Matrix4::from_scale(vec3(0.3, 0.3, 0.3)),
                        );
                        let model = state.resource_man.item_model_or_missing(stack.id);

                        extra_instances.push((instance, model, ()));
                    }
                }
            }
        }

        for (coord, data) in all_data {
            let world_coord = HEX_GRID_LAYOUT.hex_to_world_pos(**coord);
            if let Some(Data::Coord(link)) = data.get(state.resource_man.registry.data_ids.link) {
                extra_instances.push((
                    InstanceData::default()
                        .with_color_offset(colors::RED.to_linear())
                        .with_model_matrix(make_line(
                            world_coord,
                            HEX_GRID_LAYOUT.hex_to_world_pos(**link),
                            LINE_DEPTH,
                        )),
                    state.resource_man.registry.model_ids.cube1x1,
                    (),
                ));
            }

            if let Some(Data::Id(id)) = data.get(state.resource_man.registry.data_ids.item) {
                overlay_instances.push((
                    InstanceData::default().with_model_matrix(
                        Matrix4::from_translation(world_coord.extend(LINE_DEPTH))
                            * Matrix4::from_scale(vec3(0.25, 0.25, 1.0)),
                    ),
                    state.resource_man.item_model_or_missing(*id),
                    (),
                ))
            }
        }

        for (coord, (id, unit)) in render_info {
            let tile = state.resource_man.registry.tiles.get(id).unwrap();

            if let Some(theta) = all_data
                .get(coord)
                .and_then(|data| data.get(state.resource_man.registry.data_ids.direction))
                .and_then(|direction| {
                    if let Data::Coord(target) = direction {
                        math::tile_direction_to_angle(*target)
                    } else {
                        None
                    }
                })
            {
                if let Data::Color(color) = tile
                    .data
                    .get(state.resource_man.registry.data_ids.direction_color)
                    .unwrap_or(&Data::Color(colors::ORANGE))
                {
                    extra_instances.push((
                        InstanceData::default()
                            .with_color_offset(color.to_linear())
                            .with_model_matrix(
                                unit.instance.get_model_matrix()
                                    * Matrix4::from_rotation_z(theta.to_radians())
                                    * Matrix4::from_rotation_z(FRAC_PI_6 * 5.0)
                                    * Matrix4::from_scale(vec3(0.1, SQRT_3, LINE_DEPTH))
                                    * Matrix4::from_translation(vec3(0.0, 0.5, 0.0)),
                            ),
                        state.resource_man.registry.model_ids.cube1x1,
                        (),
                    ))
                }
            }
        }

        let game_instances = {
            let mut render_info = render_info.clone();

            let (bound_min, bound_max) = get_screen_world_bounding_vec(
                window::window_size_double(&renderer.gpu.window),
                camera_pos,
            );
            let (bound_min, bound_max) = (
                bound_min.as_vec2() - vec2(3.0, 3.0),
                bound_max.as_vec2() + vec2(3.0, 3.0),
            );

            for coord in culling_range.into_iter() {
                if !render_info.contains_key(&coord) {
                    let pos = HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

                    if pos.x > bound_min.x
                        && pos.x < bound_max.x
                        && pos.y > bound_min.y
                        && pos.y < bound_max.y
                    {
                        render_info.insert(
                            coord,
                            (
                                state.resource_man.registry.none,
                                RenderUnit {
                                    instance: InstanceData::default().with_model_matrix(
                                        Matrix4::from_translation(pos.extend(FAR)),
                                    ),
                                    model_override: None,
                                },
                            ),
                        );
                    }
                }
            }

            {
                let mut instances = Vec::new();

                for (coord, (id, mut unit)) in render_info {
                    let pos = HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

                    if pos.x > bound_min.x
                        && pos.x < bound_max.x
                        && pos.y > bound_min.y
                        && pos.y < bound_max.y
                    {
                        let tile = state.resource_man.registry.tiles.get(&id).unwrap();

                        if let Some(theta) = all_data
                            .get(&coord)
                            .and_then(|data| {
                                data.get(state.resource_man.registry.data_ids.direction)
                            })
                            .and_then(|direction| {
                                if let Data::Coord(target) = direction {
                                    math::tile_direction_to_angle(*target)
                                } else {
                                    None
                                }
                            })
                        {
                            unit.instance = unit
                                .instance
                                .add_model_matrix(Matrix4::from_rotation_z(theta.to_radians()));
                        }

                        if let Some(Data::Id(inactive)) = tile
                            .data
                            .get(state.resource_man.registry.data_ids.inactive_model)
                        {
                            unit.model_override =
                                Some(state.resource_man.tile_model_or_missing(*inactive));
                        }

                        if let Some(color) = tile_tints.get(&coord) {
                            unit.instance = unit.instance.with_color_offset(color.to_array())
                        }

                        let model = state.resource_man.tile_model_or_missing(id);

                        let model = state
                            .resource_man
                            .model_or_missing(unit.model_override.unwrap_or(model));

                        try_add_animation(
                            &state.resource_man,
                            state.start_instant,
                            model,
                            &mut animation_map,
                        );

                        instances.push((unit.instance, model, pos))
                    }
                }

                let camera_pos = camera_pos_float.truncate();
                instances.sort_by(|(.., a), (.., b)| {
                    camera_pos
                        .distance_squared(*a)
                        .total_cmp(&camera_pos.distance_squared(*b))
                });

                instances
                    .into_iter()
                    .rev()
                    .map(|v| (v.0, v.1, ()))
                    .collect::<Vec<_>>()
            }
        };

        for (_, model, _) in &extra_instances {
            try_add_animation(
                &state.resource_man,
                state.start_instant,
                *model,
                &mut animation_map,
            );
        }

        for (_, model, _) in &overlay_instances {
            try_add_animation(
                &state.resource_man,
                state.start_instant,
                *model,
                &mut animation_map,
            );
        }

        let r = Renderer::inner_render(
            state,
            screenshotting,
            game_instances,
            extra_instances,
            overlay_instances,
            animation_map,
        );

        gui::reset_custom_paint_state();

        r
    }

    fn inner_render(
        state: &mut GameState,
        screenshotting: bool,
        game_instances: Vec<(InstanceData, Id, ())>,
        extra_instances: Vec<(InstanceData, Id, ())>,
        overlay_instances: Vec<(InstanceData, Id, ())>,
        animation_map: AnimationMap,
    ) -> Result<(), SurfaceError> {
        let Some(renderer) = state.renderer.as_mut() else {
            return Ok(());
        };

        let camera_matrix = state.camera.get_matrix().as_mat4();
        let camera_pos = state.camera.get_pos().as_vec3();

        let size = renderer.gpu.window.inner_size();

        let game_data = gpu::indirect_instance(
            &state.resource_man,
            game_instances
                .into_iter()
                .map(|v| (v.0, camera_matrix, v.1, v.2))
                .collect(),
            &animation_map,
            true,
        );

        let extra_game_data = gpu::indirect_instance(
            &state.resource_man,
            extra_instances
                .into_iter()
                .map(|v| (v.0, camera_matrix, v.1, v.2))
                .collect(),
            &animation_map,
            true,
        );

        let overlay_game_data = gpu::indirect_instance(
            &state.resource_man,
            overlay_instances
                .into_iter()
                .map(|v| (v.0, camera_matrix, v.1, v.2))
                .collect(),
            &animation_map,
            true,
        );

        let output = renderer.gpu.surface.get_current_texture()?;

        {
            let output_size = output.texture.size();

            if output_size.width != size.width || output_size.height != size.height {
                return Ok(());
            }
        }

        let mut encoder = renderer
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let IndirectInstanceDrawData {
                opaques,
                non_opaques,
                matrix_data,
                world_matrix_data,
                draw_data,
            } = extra_game_data;

            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .extra_objects_resources
                    .opaques_instance_buffer,
                &opaques,
            );
            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .extra_objects_resources
                    .non_opaques_instance_buffer,
                &non_opaques,
            );

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Extra Objects Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(NORMAL_CLEAR),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(MODEL_DEPTH_CLEAR),
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &renderer.shared_resources.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !(draw_data.opaques.is_empty() && draw_data.non_opaques.is_empty()) {
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .extra_objects_resources
                        .uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(camera_pos, None)]),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .extra_objects_resources
                        .matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(matrix_data.into_iter().collect::<Vec<_>>().as_slice()),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .extra_objects_resources
                        .world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(
                        world_matrix_data.into_iter().collect::<Vec<_>>().as_slice(),
                    ),
                );

                render_pass.set_pipeline(&renderer.global_resources.game_pipeline);
                render_pass.set_bind_group(
                    0,
                    &renderer.render_resources.extra_objects_resources.bind_group,
                    &[],
                );
                render_pass.set_vertex_buffer(0, renderer.global_resources.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    renderer.global_resources.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .extra_objects_resources
                        .opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .extra_objects_resources
                        .non_opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.non_opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }
            }
        }

        {
            let IndirectInstanceDrawData {
                opaques,
                non_opaques,
                matrix_data,
                world_matrix_data,
                draw_data,
            } = game_data;

            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .game_resources
                    .opaques_instance_buffer,
                &opaques,
            );
            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .game_resources
                    .non_opaques_instance_buffer,
                &non_opaques,
            );

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &renderer.shared_resources.depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !(draw_data.opaques.is_empty() && draw_data.non_opaques.is_empty()) {
                renderer.gpu.queue.write_buffer(
                    &renderer.render_resources.game_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(camera_pos, None)]),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer.render_resources.game_resources.matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(matrix_data.into_iter().collect::<Vec<_>>().as_slice()),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .game_resources
                        .world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(
                        world_matrix_data.into_iter().collect::<Vec<_>>().as_slice(),
                    ),
                );

                render_pass.set_pipeline(&renderer.global_resources.game_pipeline);
                render_pass.set_bind_group(
                    0,
                    &renderer.render_resources.game_resources.bind_group,
                    &[],
                );
                render_pass.set_vertex_buffer(0, renderer.global_resources.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    renderer.global_resources.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .game_resources
                        .opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .game_resources
                        .non_opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.non_opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }
            }
        }

        {
            let IndirectInstanceDrawData {
                opaques,
                non_opaques,
                matrix_data,
                world_matrix_data,
                draw_data,
            } = overlay_game_data;

            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .overlay_objects_resources
                    .opaques_instance_buffer,
                &opaques,
            );
            gpu::update_instance_buffer(
                &renderer.gpu.device,
                &renderer.gpu.queue,
                &mut renderer
                    .render_resources
                    .overlay_objects_resources
                    .non_opaques_instance_buffer,
                &non_opaques,
            );

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Overlay Objects Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.game_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.normal_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &renderer.shared_resources.model_depth_texture().1,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &renderer.shared_resources.overlay_depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !(draw_data.opaques.is_empty() && draw_data.non_opaques.is_empty()) {
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .overlay_objects_resources
                        .uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(camera_pos, None)]),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .overlay_objects_resources
                        .matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(matrix_data.into_iter().collect::<Vec<_>>().as_slice()),
                );
                renderer.gpu.queue.write_buffer(
                    &renderer
                        .render_resources
                        .overlay_objects_resources
                        .world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(
                        world_matrix_data.into_iter().collect::<Vec<_>>().as_slice(),
                    ),
                );

                render_pass.set_pipeline(&renderer.global_resources.game_pipeline);
                render_pass.set_bind_group(
                    0,
                    &renderer
                        .render_resources
                        .overlay_objects_resources
                        .bind_group,
                    &[],
                );
                render_pass.set_vertex_buffer(0, renderer.global_resources.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    renderer.global_resources.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .overlay_objects_resources
                        .opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }

                render_pass.set_vertex_buffer(
                    1,
                    renderer
                        .render_resources
                        .overlay_objects_resources
                        .non_opaques_instance_buffer
                        .slice(..),
                );
                for (draw, _) in draw_data.non_opaques.iter() {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }
            }
        }

        {
            renderer.gpu.queue.write_buffer(
                &renderer
                    .render_resources
                    .post_processing_resources
                    .uniform_buffer,
                0,
                bytemuck::cast_slice(&[PostProcessingUBO {
                    ..Default::default()
                }]),
            );

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Post Processing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &renderer.shared_resources.game_post_processing_texture().1,
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

            render_pass.set_pipeline(&renderer.global_resources.post_processing_pipeline);
            render_pass.set_bind_group(
                0,
                renderer.shared_resources.game_post_processing_bind_group(),
                &[],
            );
            render_pass.set_bind_group(
                1,
                &renderer
                    .render_resources
                    .post_processing_resources
                    .bind_group_uniform,
                &[],
            );
            render_pass.draw(0..3, 0..1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Game Antialiasing Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &renderer.shared_resources.game_antialiasing_texture().1,
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

            render_pass.set_pipeline(&renderer.global_resources.fxaa_pipeline);
            render_pass.set_bind_group(
                0,
                renderer.shared_resources.game_antialiasing_bind_group(),
                &[],
            );
            render_pass.draw(0..3, 0..1);
        }

        let custom_gui_commands: CommandBuffer;
        {
            let surface = SurfaceInfo {
                format: renderer.gpu.config.format,
                sample_count: 4,
                color_attachments: vec![Some(RenderPassColorAttachment {
                    view: &renderer.shared_resources.gui_texture().1,
                    resolve_target: Some(&renderer.shared_resources.gui_texture_resolve().1),
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_format: None,
                depth_attachment: None,
                depth_load_op: None,
            };

            let resources: &mut YakuiRenderResources = &mut (
                state.resource_man.clone(),
                renderer.global_resources.clone(),
                renderer.render_resources.gui_resources.take(),
                surface.format,
                animation_map,
                Some(Vec::new()),
                Default::default(),
                Default::default(),
                Default::default(),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("yakui Render Pass"),
                    color_attachments: &surface.color_attachments,
                    depth_stencil_attachment: None,
                    ..Default::default()
                });

                let gui = state.gui.as_mut().unwrap();

                custom_gui_commands = gui.renderer.paint_with::<GameElementPaint>(
                    &mut gui.yak,
                    &renderer.gpu.device,
                    &renderer.gpu.queue,
                    &mut render_pass,
                    surface,
                    resources,
                );
            }

            renderer.render_resources.gui_resources = resources.2.take();
        };

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Combine Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &renderer.shared_resources.first_combine_texture().1,
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

            render_pass.set_pipeline(&renderer.global_resources.combine_pipeline);
            render_pass.set_bind_group(
                0,
                renderer.shared_resources.first_combine_bind_group(),
                &[],
            );
            render_pass.draw(0..3, 0..1)
        }

        {
            let view = output
                .texture
                .create_view(&TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
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

            render_pass.set_pipeline(&renderer.global_resources.present_pipeline);
            render_pass.set_bind_group(0, renderer.shared_resources.present_bind_group(), &[]);
            render_pass.draw(0..3, 0..1)
        }

        fn size_align(size: u32, alignment: u32) -> u32 {
            ((size + alignment - 1) / alignment) * alignment
        }

        let block_size = output.texture.format().block_copy_size(None).unwrap();
        let texture_dim = output.texture.size();
        let buffer_dim = texture_dim.physical_size(output.texture.format());
        let padded_width = size_align(buffer_dim.width * block_size, COPY_BYTES_PER_ROW_ALIGNMENT);

        let screenshot_buffer = if screenshotting {
            let intermediate_texture = renderer.gpu.device.create_texture(&TextureDescriptor {
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
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
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

                render_pass.set_pipeline(&renderer.global_resources.screenshot_pipeline);
                render_pass.set_bind_group(
                    0,
                    renderer.shared_resources.screenshot_bind_group(),
                    &[],
                );
                render_pass.draw(0..3, 0..1);
            }

            let buffer = renderer.gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Screenshot Buffer"),
                size: size_align(
                    padded_width * buffer_dim.height,
                    COPY_BUFFER_ALIGNMENT as u32,
                ) as BufferAddress,
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

        renderer
            .gpu
            .queue
            .submit([custom_gui_commands, encoder.finish()]);

        if let Some(buffer) = screenshot_buffer {
            {
                let slice = buffer.slice(..);

                let (tx, rx) = oneshot::channel();

                slice.map_async(MapMode::Read, move |result| {
                    tx.send(result).unwrap();
                });
                renderer.gpu.device.poll(Maintain::Wait);
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
                    renderer
                        .screenshot_clipboard
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

        renderer.gpu.window.pre_present_notify();

        output.present();

        Ok(())
    }
}
