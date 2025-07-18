use std::{
    borrow::Cow,
    collections::{BTreeMap, VecDeque},
    mem,
    ops::Mul,
    sync::Arc,
    time::Instant,
};

use arboard::{Clipboard, ImageData};
use automancy_defs::{
    coord::TileCoord,
    id::{Id, ModelId, RenderTagId},
    math::{Matrix4, Vec2, Vec3, Vec4},
    rendering::{
        AnimationMatrixData, GameMatrix, GameUBO, GpuInstance, InstanceData, MatrixData,
        PostProcessingUBO, WorldMatrixData,
    },
};
use automancy_resources::{ResourceManager, rhai_render::RenderCommand};
use automancy_system::{GameGui, game::GameSystemMessage};
use automancy_ui::{GameElementPaint, UiGameObjectType};
use hashbrown::{HashMap, HashSet};
use image::{EncodableLayout, RgbaImage};
use ordermap::OrderMap;
use range_set_blaze::RangeSetBlaze;
use slice_group_by::GroupBy;
use wgpu::{
    BufferDescriptor, BufferUsages, Color, CommandBuffer, CommandEncoderDescriptor, Extent3d,
    IndexFormat, LoadOp, MapMode, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, SurfaceError,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    util::DrawIndexedIndirectArgs,
};
use yakui::{Rect, UVec2};
use yakui_wgpu::SurfaceInfo;

use crate::{
    GameState, gpu,
    gpu::{
        GlobalResources, Gpu, GuiResources, MODEL_DEPTH_CLEAR, NORMAL_CLEAR, RenderResources,
        SCREENSHOT_FORMAT, SharedResources,
    },
};

pub type OverlayInstance = (InstanceData, ModelId, GameMatrix<true>, usize);
pub type GuiInstance = (
    UiGameObjectType,
    InstanceData,
    GameMatrix<false>,
    (usize, Vec2),
);

pub type AnimationCache = HashMap<ModelId, HashMap<usize, Matrix4>>;
pub type AnimationMatrixDataMap = OrderMap<(ModelId, usize), AnimationMatrixData>;

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

    pub animation_cache: AnimationCache,
}

const WE_ONLY_USE_1_WORLD_MATRIX_IN_GAME_LOL: u32 = 0;

pub struct GameRenderer {
    pub gpu: Gpu,
    pub shared_resources: SharedResources,
    pub render_resources: RenderResources,
    pub global_resources: Arc<GlobalResources>,

    pub overlay_instances: Vec<OverlayInstance>,

    pub tile_tints: HashMap<TileCoord, Vec4>,
    last_tile_tints: HashMap<TileCoord, Vec4>,

    pub take_item_animations: HashMap<Id, VecDeque<(Instant, Rect)>>,

    object_ids: OrderMap<(TileCoord, RenderTagId, ModelId, usize), ()>,
    coord_to_keys: HashMap<TileCoord, HashSet<(RenderTagId, ModelId, usize)>>,

    instance_ranges: BTreeMap<(ModelId, usize), RangeSetBlaze<usize>>,
    instances: Vec<GpuInstance>,
    matrix_data_map: Vec<MatrixData>,
    animation_matrix_data_map: AnimationMatrixDataMap,

    gui_opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,
    gui_non_opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,
    gui_animation_matrix_data_map: Option<AnimationMatrixDataMap>,

    gui_packed_size: Option<UVec2>,
    gui_rects: Vec<Option<crunch::Rect>>,

    animation_cache: AnimationCache,
    screenshot_clipboard: Clipboard,
}

impl GameRenderer {
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

            tile_tints: Default::default(),
            last_tile_tints: Default::default(),
            overlay_instances: Default::default(),

            take_item_animations: Default::default(),

            object_ids: Default::default(),
            coord_to_keys: Default::default(),

            instance_ranges: Default::default(),
            instances: Default::default(),
            matrix_data_map: Default::default(),
            animation_matrix_data_map: Default::default(),

            gui_opaque_draws: Some(Default::default()),
            gui_non_opaque_draws: Some(Default::default()),
            gui_animation_matrix_data_map: Some(Default::default()),

            gui_packed_size: Default::default(),
            gui_rects: Default::default(),

            animation_cache: AnimationCache::new(),
            screenshot_clipboard: Clipboard::new().unwrap(),
        }
    }
}

pub fn try_add_animation(
    resource_man: &ResourceManager,
    start_instant: Instant,
    model: ModelId,
    cache: &mut AnimationCache,
) {
    if !cache.contains_key(&model) {
        if let Some((_, anims)) = resource_man.all_meshes_anims.get(&model) {
            let elapsed = Instant::now().duration_since(start_instant).as_secs_f32();

            let anims = anims
                .iter()
                .map(|anim| {
                    let last = anim.inputs.last().unwrap();
                    let wrapped = elapsed % last;
                    let index = anim.inputs.partition_point(|&v| v < wrapped);

                    (anim.target, anim.outputs[index])
                })
                .collect::<Vec<_>>();

            let anims = anims
                .binary_group_by_key(|(target, _)| *target)
                .map(|grouped| {
                    (
                        grouped[0].0, // target
                        grouped
                            .iter()
                            .map(|v| v.1)
                            .fold(Matrix4::IDENTITY, Mul::mul),
                    )
                })
                .collect::<HashMap<_, _>>();

            cache.insert(model, anims);
        }
    }
}

pub fn render(state: &mut GameState, screenshotting: bool) -> Result<(), SurfaceError> {
    let Some(renderer) = state.renderer.as_mut() else {
        return Ok(());
    };

    let size = renderer.gpu.window.inner_size();

    if size.width == 0 || size.height == 0 {
        return Ok(());
    }

    renderer.animation_cache.clear();

    let last_tile_tints = mem::take(&mut renderer.last_tile_tints);
    let tile_tints = mem::take(&mut renderer.tile_tints);

    let camera_pos = state.camera.get_pos();
    let culling_range = state.camera.culling_range;

    let render_commands = {
        let game = state.game.clone();

        state
            .tokio
            .block_on(game.call(
                |reply| GameSystemMessage::GetAllRenderCommands {
                    reply,
                    culling_range,
                },
                None,
            ))
            .unwrap()
            .unwrap()
    };

    let mut instances_changes = HashSet::new();
    let mut matrix_data_changes = HashSet::new();

    for batch in render_commands {
        let mut batch = batch.into_iter().collect::<Vec<_>>();
        batch.sort_by_key(|v| v.0.ulength());

        let mut untrack_commands = BTreeMap::new();
        let mut track_commands = BTreeMap::new();
        let mut transform_commands = BTreeMap::new();

        for (coord, commands) in batch {
            for command in commands {
                match command {
                    RenderCommand::Untrack { tag, model } => {
                        untrack_commands
                            .entry(model)
                            .or_insert_with(Vec::new)
                            .push((coord, tag));
                    }
                    RenderCommand::Track { tag, model } => {
                        track_commands
                            .entry(model)
                            .or_insert_with(Vec::new)
                            .push((coord, tag));
                    }
                    RenderCommand::Transform {
                        tag,
                        model,
                        model_matrix,
                    } => {
                        transform_commands
                            .entry(model)
                            .or_insert_with(Vec::new)
                            .push((coord, tag, model_matrix));
                    }
                }
            }
        }

        for (model, commands) in untrack_commands {
            let (model, (meshes, ..)) = state.resource_man.mesh_or_missing_tile_mesh(&model);

            for mesh in meshes.iter().flatten() {
                for (coord, tag) in commands.iter().cloned() {
                    let swapping_index = renderer.object_ids.last_entry().map(|v| v.index());
                    let swapping_key = renderer.object_ids.last_entry().map(|v| *v.key());

                    let (removed_index, ..) = renderer
                        .object_ids
                        .swap_remove_full(&(coord, tag, model, mesh.index))
                        .expect("render object id wasn't tracked");

                    let swapping_index = swapping_index.unwrap_or(removed_index);

                    if let Some(keys) = renderer.coord_to_keys.get_mut(&coord) {
                        assert!(
                            keys.remove(&(tag, model, mesh.index)),
                            "key set in 'coord to keys map' didn't have this key"
                        );
                    }

                    {
                        renderer.matrix_data_map[removed_index] = Default::default();
                        renderer.matrix_data_map.swap(removed_index, swapping_index);

                        matrix_data_changes.insert(swapping_index);
                        matrix_data_changes.insert(removed_index);
                    }
                    {
                        renderer.instances[removed_index] = Default::default();
                        renderer.instances.swap(removed_index, swapping_index);

                        instances_changes.insert(swapping_index);
                        instances_changes.insert(removed_index);
                    }

                    {
                        renderer
                            .instance_ranges
                            .entry((model, mesh.index))
                            .or_default()
                            .remove(removed_index);

                        if swapping_index != removed_index {
                            if let Some((.., model, mesh_index)) = swapping_key {
                                renderer
                                    .instance_ranges
                                    .entry((model, mesh_index))
                                    .or_default()
                                    .insert(removed_index);
                                renderer
                                    .instance_ranges
                                    .entry((model, mesh_index))
                                    .or_default()
                                    .remove(swapping_index);
                            }
                        }
                    }

                    if swapping_index != removed_index {
                        renderer.instances[removed_index].matrix_index = removed_index as u32;
                    }
                }
            }
        }

        for (model, commands) in track_commands {
            let (model, (meshes, ..)) = state.resource_man.mesh_or_missing_tile_mesh(&model);

            for mesh in meshes.iter().flatten() {
                for (coord, tag) in commands.iter().cloned() {
                    if !renderer
                        .animation_matrix_data_map
                        .contains_key(&(model, mesh.index))
                    {
                        renderer
                            .animation_matrix_data_map
                            .insert((model, mesh.index), AnimationMatrixData::default());
                    }
                    let animation_matrix_index = renderer
                        .animation_matrix_data_map
                        .get_index_of(&(model, mesh.index))
                        .unwrap();

                    let (index, prev_id_slot) = renderer
                        .object_ids
                        .insert_full((coord, tag, model, mesh.index), ());
                    assert!(
                        prev_id_slot.is_none(),
                        "render object id was already tracked"
                    );

                    assert!(
                        renderer
                            .coord_to_keys
                            .entry(coord)
                            .or_default()
                            .insert((tag, model, mesh.index)),
                        "coord to keys map already has the same key"
                    );

                    renderer
                        .matrix_data_map
                        .resize_with(index + 1, Default::default);
                    renderer.matrix_data_map[index] = MatrixData::default();
                    matrix_data_changes.insert(index);

                    renderer.instances.resize_with(index + 1, Default::default);
                    renderer.instances[index] = GpuInstance {
                        matrix_index: index as u32,
                        animation_matrix_index: animation_matrix_index as u32,
                        world_matrix_index: WE_ONLY_USE_1_WORLD_MATRIX_IN_GAME_LOL,
                        color_offset: [0.0; 4],
                        alpha: 1.0,
                    };
                    instances_changes.insert(index);

                    renderer
                        .instance_ranges
                        .entry((model, mesh.index))
                        .or_default()
                        .insert(index);
                }
            }
        }

        for (model, commands) in transform_commands {
            let (model, (meshes, ..)) = state.resource_man.mesh_or_missing_tile_mesh(&model);

            for mesh in meshes.iter().flatten() {
                for (coord, tag, model_matrix) in commands.iter().cloned() {
                    if let Some(index) = renderer
                        .object_ids
                        .get_index_of(&(coord, tag, model, mesh.index))
                    {
                        if let Some(matrix) = renderer.matrix_data_map.get_mut(index) {
                            *matrix = MatrixData::new(model_matrix, mesh.matrix);

                            matrix_data_changes.insert(index);
                        }
                    }
                }
            }
        }
    }

    let overlay_instances = mem::take(&mut renderer.overlay_instances);
    for &(_, model, _, mesh_index) in &overlay_instances {
        if !renderer
            .animation_matrix_data_map
            .contains_key(&(model, mesh_index))
        {
            renderer
                .animation_matrix_data_map
                .insert((model, mesh_index), AnimationMatrixData::default());
        }
    }

    for (model, _) in renderer.animation_matrix_data_map.keys() {
        try_add_animation(
            &state.resource_man,
            state.start_instant,
            *model,
            &mut renderer.animation_cache,
        );
    }

    for (&model, anim) in &renderer.animation_cache {
        for (&mesh_id, &matrix) in anim {
            if let Some(data) = renderer
                .animation_matrix_data_map
                .get_mut(&(model, mesh_id))
            {
                data.animation_matrix = matrix.to_cols_array_2d();
            }
        }
    }

    {
        for (coord, _) in last_tile_tints {
            if tile_tints.contains_key(&coord) {
                continue;
            };

            let Some(keys) = renderer.coord_to_keys.get(&coord) else {
                continue;
            };

            for &key in keys {
                let index = renderer
                    .object_ids
                    .get_index_of(&(coord, key.0, key.1, key.2))
                    .unwrap();

                renderer.instances[index].color_offset = [0.0; 4];
                instances_changes.insert(index);
            }
        }

        for (coord, tint) in &tile_tints {
            let Some(keys) = renderer.coord_to_keys.get(coord) else {
                continue;
            };

            for &key in keys {
                let index = renderer
                    .object_ids
                    .get_index_of(&(*coord, key.0, key.1, key.2))
                    .unwrap();

                renderer.instances[index].color_offset = tint.to_array();
                instances_changes.insert(index);
            }
        }
    }

    let mut instances_changes = instances_changes.into_iter().collect::<Vec<_>>();
    instances_changes.sort();
    let mut matrix_data_changes = matrix_data_changes.into_iter().collect::<Vec<_>>();
    matrix_data_changes.sort();

    let r = renderer.inner_render(
        state.resource_man.clone(),
        state.gui.as_mut().unwrap(),
        camera_pos,
        state.camera.get_matrix(),
        instances_changes,
        matrix_data_changes,
        overlay_instances,
        screenshotting,
    );

    automancy_ui::custom::reset_paint_state();
    renderer.last_tile_tints = tile_tints;

    r
}

impl GameRenderer {
    fn inner_render(
        &mut self,
        resource_man: Arc<ResourceManager>,
        gui: &mut GameGui<YakuiRenderResources>,
        camera_pos: Vec3,
        camera_matrix: Matrix4,
        instances_changes: Vec<usize>,
        matrix_data_changes: Vec<usize>,
        overlay_instances: Vec<OverlayInstance>,
        screenshotting: bool,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();

        let surface = self.gpu.surface.get_current_texture()?;

        {
            let output_size = surface.texture.size();

            if output_size.width != size.width || output_size.height != size.height {
                return Ok(());
            }
        }

        let surface_pixel_size = surface.texture.format().target_pixel_byte_cost().unwrap();
        let surface_dim = surface.texture.size();
        let buffer_dim = {
            let dim = surface_dim.physical_size(surface.texture.format());

            let padded_width = dim.width.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

            Extent3d {
                width: padded_width,
                ..dim
            }
        };
        let buffer_size = (buffer_dim.width * buffer_dim.height * surface_pixel_size)
            .div_ceil(wgpu::COPY_BUFFER_ALIGNMENT)
            * wgpu::COPY_BUFFER_ALIGNMENT;

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut game_staging_belts = [None, None];

        {
            if !self.instances.is_empty() {
                game_staging_belts[0] = gpu::resize_update_buffer_with_changes(
                    &mut encoder,
                    &self.gpu.device,
                    &mut self.render_resources.game_resources.instance_buffer,
                    &instances_changes,
                    &self.instances,
                );
                game_staging_belts[1] = gpu::update_buffer_with_changes(
                    &mut encoder,
                    &self.gpu.device,
                    &self.render_resources.game_resources.matrix_data_buffer,
                    &matrix_data_changes,
                    &self.matrix_data_map,
                );
                gpu::ordered_map_update_buffer(
                    &self.gpu.queue,
                    &self
                        .render_resources
                        .game_resources
                        .animation_matrix_data_buffer,
                    &self.animation_matrix_data_map,
                );
                self.gpu.queue.write_buffer(
                    &self
                        .render_resources
                        .game_resources
                        .world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(&[WorldMatrixData::new(camera_matrix)]),
                );
                self.gpu.queue.write_buffer(
                    &self.render_resources.game_resources.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(camera_pos, None)]),
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Game Render Pass"),
                        color_attachments: &[
                            Some(RenderPassColorAttachment {
                                view: &self.shared_resources.game_texture().1,
                                depth_slice: None,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Clear(Color::BLACK),
                                    store: StoreOp::Store,
                                },
                            }),
                            Some(RenderPassColorAttachment {
                                view: &self.shared_resources.normal_texture().1,
                                depth_slice: None,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Clear(NORMAL_CLEAR),
                                    store: StoreOp::Store,
                                },
                            }),
                            Some(RenderPassColorAttachment {
                                view: &self.shared_resources.model_depth_texture().1,
                                depth_slice: None,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Clear(MODEL_DEPTH_CLEAR),
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

                    render_pass.set_pipeline(&self.global_resources.game_pipeline);
                    render_pass.set_bind_group(
                        0,
                        &self.render_resources.game_resources.bind_group,
                        &[],
                    );
                    render_pass.set_vertex_buffer(
                        1,
                        self.render_resources
                            .game_resources
                            .instance_buffer
                            .slice(..),
                    );
                    render_pass.set_vertex_buffer(0, self.global_resources.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        self.global_resources.index_buffer.slice(..),
                        IndexFormat::Uint16,
                    );

                    for (&(model, mesh_index), ranges) in &self.instance_ranges {
                        let (meshes, ..) = resource_man.all_meshes_anims.get(&model).unwrap();

                        if let Some(mesh) = &meshes[mesh_index] {
                            if mesh.opaque {
                                let index_range =
                                    &resource_man.all_index_ranges[&model][&mesh.index];

                                for range in ranges.ranges() {
                                    render_pass.draw_indexed(
                                        index_range.pos..(index_range.pos + index_range.count),
                                        index_range.base_vertex,
                                        (*range.start() as u32)..(*range.end() as u32 + 1),
                                    );
                                }
                            }
                        }
                    }

                    for (&(model, mesh_index), ranges) in &self.instance_ranges {
                        let (meshes, ..) = resource_man.all_meshes_anims.get(&model).unwrap();

                        if let Some(mesh) = &meshes[mesh_index] {
                            if !mesh.opaque {
                                let index_range =
                                    &resource_man.all_index_ranges[&model][&mesh.index];

                                for range in ranges.ranges() {
                                    render_pass.draw_indexed(
                                        index_range.pos..(index_range.pos + index_range.count),
                                        index_range.base_vertex,
                                        (*range.start() as u32)..(*range.end() as u32 + 1),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Overlay Objects Render Pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.game_texture().1,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.normal_texture().1,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.shared_resources.model_depth_texture().1,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shared_resources.overlay_depth_texture().1,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !overlay_instances.is_empty() {
                gpu::resize_update_buffer(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut self
                        .render_resources
                        .overlay_objects_resources
                        .instance_buffer,
                    &overlay_instances
                        .iter()
                        .enumerate()
                        .map(|(idx, (v, model_id, _, mesh_index))| {
                            let animation_index = self
                                .animation_matrix_data_map
                                .get_index_of(&(*model_id, *mesh_index))
                                .unwrap();

                            GpuInstance {
                                color_offset: v.color_offset,
                                alpha: v.alpha,
                                matrix_index: idx as u32,
                                world_matrix_index: idx as u32,
                                animation_matrix_index: animation_index as u32,
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                self.gpu.queue.write_buffer(
                    &self
                        .render_resources
                        .overlay_objects_resources
                        .matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(
                        &overlay_instances
                            .iter()
                            .map(|v| MatrixData::new(v.2.model_matrix(), v.2.mesh_matrix()))
                            .collect::<Vec<_>>(),
                    ),
                );
                self.gpu.queue.write_buffer(
                    &self
                        .render_resources
                        .overlay_objects_resources
                        .world_matrix_data_buffer,
                    0,
                    bytemuck::cast_slice(
                        &overlay_instances
                            .iter()
                            .map(|v| WorldMatrixData::new(v.2.world_matrix()))
                            .collect::<Vec<_>>(),
                    ),
                );
                self.gpu.queue.write_buffer(
                    &self
                        .render_resources
                        .overlay_objects_resources
                        .uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[GameUBO::new(camera_pos, None)]),
                );

                render_pass.set_pipeline(&self.global_resources.game_pipeline);
                render_pass.set_bind_group(
                    0,
                    &self.render_resources.overlay_objects_resources.bind_group,
                    &[],
                );
                render_pass.set_vertex_buffer(
                    1,
                    self.render_resources
                        .overlay_objects_resources
                        .instance_buffer
                        .slice(..),
                );
                render_pass.set_vertex_buffer(0, self.global_resources.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.global_resources.index_buffer.slice(..),
                    IndexFormat::Uint16,
                );

                for (i, (_, model, _, mesh_index)) in overlay_instances.into_iter().enumerate() {
                    let i = i as u32;

                    let range = &resource_man.all_index_ranges[&model][&mesh_index];

                    render_pass.draw_indexed(
                        range.pos..(range.pos + range.count),
                        range.base_vertex,
                        i..(i + 1),
                    );
                }
            }
        }

        {
            self.gpu.queue.write_buffer(
                &self
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
                    view: &self.shared_resources.game_post_processing_texture().1,
                    depth_slice: None,
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

            render_pass.set_pipeline(&self.global_resources.post_processing_pipeline);
            render_pass.set_bind_group(
                0,
                self.shared_resources.game_post_processing_bind_group(),
                &[],
            );
            render_pass.set_bind_group(
                1,
                &self
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
                    view: &self.shared_resources.game_antialiasing_texture().1,
                    depth_slice: None,
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

            render_pass.set_pipeline(&self.global_resources.fxaa_pipeline);
            render_pass.set_bind_group(
                0,
                self.shared_resources.game_antialiasing_bind_group(),
                &[],
            );
            render_pass.draw(0..3, 0..1);
        }

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
                rects: mem::take(&mut self.gui_rects),

                animation_cache: mem::take(&mut self.animation_cache),
            };

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("yakui Render Pass"),
                    color_attachments: &surface_info.color_attachments,
                    depth_stencil_attachment: None,
                    ..Default::default()
                });

                custom_gui_commands = gui.renderer.paint_with::<GameElementPaint>(
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

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Combine Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.shared_resources.first_combine_texture().1,
                    depth_slice: None,
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

            render_pass.set_pipeline(&self.global_resources.combine_pipeline);
            render_pass.set_bind_group(0, self.shared_resources.first_combine_bind_group(), &[]);
            render_pass.draw(0..3, 0..1)
        }

        {
            let view = surface
                .texture
                .create_view(&TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Present Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
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

            render_pass.set_pipeline(&self.global_resources.present_pipeline);
            render_pass.set_bind_group(0, self.shared_resources.present_bind_group(), &[]);
            render_pass.draw(0..3, 0..1)
        }

        let screenshot_buffer = if screenshotting {
            let intermediate_texture = self.gpu.device.create_texture(&TextureDescriptor {
                label: Some("Screenshot Intermediate Texture"),
                size: surface_dim,
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
                        depth_slice: None,
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

                render_pass.set_pipeline(&self.global_resources.screenshot_pipeline);
                render_pass.set_bind_group(0, self.shared_resources.screenshot_bind_group(), &[]);
                render_pass.draw(0..3, 0..1);
            }

            let buffer = self.gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Screenshot Buffer"),
                size: buffer_size,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                intermediate_texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(buffer_dim.width * surface_pixel_size),
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
            .submit([custom_gui_commands, encoder.finish()]);

        for mut belt in game_staging_belts.into_iter().flatten() {
            belt.recall();
        }

        if let Some(buffer) = screenshot_buffer {
            let slice = buffer.slice(..);
            slice.map_async(MapMode::Read, move |result| {
                result.unwrap();
            });

            self.gpu.device.poll(wgpu::PollType::Wait);

            let padded_data = slice.get_mapped_range().to_vec();
            let mut data = Vec::new();

            let padded_width = (buffer_dim.width * surface_pixel_size) as usize;
            let unpadded_width = (surface_dim.width * surface_pixel_size) as usize;

            for chunk in padded_data.chunks(padded_data) {
                data.extend(&chunk[..unpadded_width]);
            }

            if let Some(image) = RgbaImage::from_vec(size.width, size.height, data) {
                self.screenshot_clipboard
                    .set_image(ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        bytes: Cow::from(image.as_bytes()),
                    })
                    .unwrap();
            }
        }

        self.gpu.window.pre_present_notify();

        surface.present();

        Ok(())
    }
}
