use std::sync::Arc;
use std::time::Instant;
use std::{borrow::Cow, collections::BTreeMap};
use std::{collections::BTreeSet, mem};
use std::{collections::VecDeque, ops::Mul};

use arboard::{Clipboard, ImageData};
use hashbrown::{HashMap, HashSet};
use image::{EncodableLayout, RgbaImage};
use ordermap::OrderMap;
use tokio::sync::oneshot;
use wgpu::{
    util::DrawIndexedIndirectArgs, BufferAddress, BufferDescriptor, BufferUsages, Color,
    CommandEncoderDescriptor, ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain,
    MapMode, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, SurfaceError, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, COPY_BUFFER_ALIGNMENT, COPY_BYTES_PER_ROW_ALIGNMENT,
};
use wgpu::{CommandBuffer, StoreOp};

use automancy_defs::math::Matrix4;
use automancy_defs::rendering::{GameUBO, InstanceData};
use automancy_defs::{
    coord::TileCoord,
    math::{Vec2, Vec4},
    rendering::AnimationMatrixData,
};
use automancy_defs::{id::Id, rendering::GameMatrix};
use automancy_defs::{id::ModelId, math::Vec3};
use automancy_defs::{id::RenderTagId, rendering::PostProcessingUBO};
use automancy_defs::{
    rendering::{GpuInstance, MatrixData, WorldMatrixData},
    slice_group_by::GroupBy,
};
use automancy_resources::rhai_render::RenderCommand;
use automancy_resources::ResourceManager;
use yakui::{Rect, UVec2};
use yakui_wgpu::SurfaceInfo;

use crate::GameState;
use crate::{
    game::GameSystemMessage,
    gui::{GameElementPaint, Gui},
};
use crate::{gpu, gui};
use crate::{
    gpu::{
        GlobalResources, Gpu, GuiResources, RenderResources, SharedResources, MODEL_DEPTH_CLEAR,
        NORMAL_CLEAR, SCREENSHOT_FORMAT,
    },
    gui::UiGameObjectType,
};

pub type OverlayInstance = (InstanceData, ModelId, GameMatrix<true>, usize);
pub type GuiInstance = (
    UiGameObjectType,
    InstanceData,
    GameMatrix<false>,
    (usize, Vec2),
);

pub type InstanceMap = BTreeMap<u32, GpuInstance>;
pub type DrawInfoMap = BTreeMap<u32, DrawIndexedIndirectArgs>;
pub type MatrixDataMap = BTreeMap<u32, MatrixData>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ObjectType {
    Opaque,
    NonOpaque,
}

const WE_ONLY_USE_1_WORLD_MATRIX_IN_GAME_LOL: u32 = 0;

pub struct Renderer {
    pub gpu: Gpu,
    pub shared_resources: SharedResources,
    pub render_resources: RenderResources,
    pub global_resources: Arc<GlobalResources>,

    pub overlay_instances: Vec<OverlayInstance>,

    pub tile_tints: HashMap<TileCoord, Vec4>,
    last_tile_tints: HashMap<TileCoord, Vec4>,

    pub take_item_animations: HashMap<Id, VecDeque<(Instant, Rect)>>,

    object_ids: OrderMap<(TileCoord, RenderTagId, ModelId, usize), ObjectType>,
    coord_to_keys: HashMap<TileCoord, HashSet<(RenderTagId, ModelId, usize)>>,

    instances: InstanceMap,
    opaque_draws: DrawInfoMap,
    non_opaque_draws: DrawInfoMap,
    matrix_data_map: MatrixDataMap,
    animation_matrix_data_map: AnimationMatrixDataMap,

    gui_opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,
    gui_non_opaque_draws: Option<Vec<(DrawIndexedIndirectArgs, usize)>>,
    gui_animation_matrix_data_map: Option<AnimationMatrixDataMap>,

    gui_packed_size: Option<UVec2>,
    gui_rects: Vec<Option<crunch::Rect>>,

    animation_cache: AnimationCache,
    screenshot_clipboard: Clipboard,
}

impl Renderer {
    pub fn reset_buffers(&mut self) {
        self.object_ids.clear();
        self.coord_to_keys.clear();

        self.instances = Default::default();
        gpu::clear_buffer(
            &self.gpu.device,
            &mut self.render_resources.game_resources.instance_buffer,
        );

        self.opaque_draws = Default::default();

        self.non_opaque_draws = Default::default();

        self.matrix_data_map = Default::default();

        self.animation_cache = Default::default();
    }

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

            instances: Default::default(),
            opaque_draws: Default::default(),
            non_opaque_draws: Default::default(),
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
        if let Some((_, anims)) = resource_man.all_models.get(&model) {
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
    let mut draws_changes = HashSet::new();

    for batch in render_commands {
        for (coord, commands) in batch {
            for command in commands {
                match command {
                    RenderCommand::Untrack { tag, model } => {
                        let (model, (meshes, ..)) =
                            state.resource_man.mesh_or_missing_tile_mesh(&model);

                        for mesh in meshes.iter().flatten() {
                            let swapping_index =
                                renderer.object_ids.last_entry().map(|v| v.index());

                            let swapping_type = renderer.object_ids.last_entry().map(|v| *v.get());

                            let (removed_index, .., removed_type) = renderer
                                .object_ids
                                .swap_remove_full(&(coord, tag, model, mesh.index))
                                .expect("render object id wasn't tracked");

                            let swapping_index = swapping_index.unwrap_or(removed_index);
                            let swapping_type = swapping_type.unwrap_or(removed_type);

                            let removed_index: u32 = removed_index.try_into().unwrap();
                            let swapping_index: u32 = swapping_index.try_into().unwrap();

                            if let Some(keys) = renderer.coord_to_keys.get_mut(&coord) {
                                assert!(
                                    keys.remove(&(tag, model, mesh.index)),
                                    "key set in 'coord to keys map' didn't have this key"
                                );
                            }

                            {
                                let opaque = removed_type == ObjectType::Opaque;

                                let draws = if opaque {
                                    &mut renderer.opaque_draws
                                } else {
                                    &mut renderer.non_opaque_draws
                                };

                                renderer
                                    .matrix_data_map
                                    .remove(&removed_index)
                                    .expect("matrix data wasn't tracked");
                                matrix_data_changes.insert(removed_index);

                                renderer
                                    .instances
                                    .remove(&removed_index)
                                    .expect("instance data wasn't tracked");
                                instances_changes.insert(removed_index);

                                draws
                                    .remove(&removed_index)
                                    .expect("draw data wasn't tracked");
                                draws_changes.insert(removed_index);
                            }

                            {
                                let opaque = swapping_type == ObjectType::Opaque;

                                let draws = if opaque {
                                    &mut renderer.opaque_draws
                                } else {
                                    &mut renderer.non_opaque_draws
                                };

                                if swapping_index != removed_index {
                                    {
                                        let matrix = renderer
                                            .matrix_data_map
                                            .remove(&swapping_index)
                                            .unwrap();

                                        matrix_data_changes.insert(swapping_index);
                                        assert!(renderer
                                            .matrix_data_map
                                            .insert(removed_index, matrix)
                                            .is_none());
                                    }
                                    {
                                        let mut instance =
                                            renderer.instances.remove(&swapping_index).unwrap();
                                        instances_changes.insert(swapping_index);

                                        instance.matrix_index = removed_index;
                                        assert!(renderer
                                            .instances
                                            .insert(removed_index, instance)
                                            .is_none());
                                    }

                                    {
                                        let mut draw = draws.remove(&swapping_index).unwrap();
                                        draws_changes.insert(swapping_index);

                                        draw.first_instance = removed_index;

                                        assert!(draws.insert(removed_index, draw).is_none());
                                    }
                                }
                            }
                        }
                    }
                    RenderCommand::Track { tag, model } => {
                        let (model, (meshes, ..)) =
                            state.resource_man.mesh_or_missing_tile_mesh(&model);

                        for mesh in meshes.iter().flatten() {
                            let opaque = mesh.opaque;

                            let draws = if opaque {
                                &mut renderer.opaque_draws
                            } else {
                                &mut renderer.non_opaque_draws
                            };

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

                            let (index, prev_id_slot) = renderer.object_ids.insert_full(
                                (coord, tag, model, mesh.index),
                                if opaque {
                                    ObjectType::Opaque
                                } else {
                                    ObjectType::NonOpaque
                                },
                            );
                            assert!(
                                prev_id_slot.is_none(),
                                "render object id was already tracked"
                            );
                            let index: u32 = index.try_into().unwrap();

                            assert!(
                                renderer
                                    .coord_to_keys
                                    .entry(coord)
                                    .or_default()
                                    .insert((tag, model, mesh.index)),
                                "coord to keys map already has the same key"
                            );

                            assert!(
                                renderer
                                    .matrix_data_map
                                    .insert(index, MatrixData::default())
                                    .is_none(),
                                "matrix data was already tracked",
                            );
                            matrix_data_changes.insert(index);

                            assert!(
                                renderer
                                    .instances
                                    .insert(
                                        index,
                                        GpuInstance {
                                            matrix_index: index,
                                            animation_matrix_index: animation_matrix_index as u32,
                                            world_matrix_index:
                                                WE_ONLY_USE_1_WORLD_MATRIX_IN_GAME_LOL,
                                            color_offset: [0.0; 4],
                                            alpha: 1.0,
                                        },
                                    )
                                    .is_none(),
                                "instance data was already tracked",
                            );
                            instances_changes.insert(index);

                            let index_range =
                                &state.resource_man.all_index_ranges[&model][&mesh.index];

                            assert!(
                                draws
                                    .insert(
                                        index,
                                        DrawIndexedIndirectArgs {
                                            first_index: index_range.pos,
                                            index_count: index_range.count,
                                            base_vertex: index_range.base_vertex,
                                            instance_count: 1,
                                            first_instance: index,
                                        },
                                    )
                                    .is_none(),
                                "draw data was already tracked",
                            );
                            draws_changes.insert(index);
                        }
                    }
                    RenderCommand::Transform {
                        tag,
                        model,
                        model_matrix,
                    } => {
                        let (model, (meshes, ..)) =
                            state.resource_man.mesh_or_missing_tile_mesh(&model);

                        for mesh in meshes.iter().flatten() {
                            if let Some(index) = renderer
                                .object_ids
                                .get_index_of(&(coord, tag, model, mesh.index))
                            {
                                let index: u32 = index.try_into().unwrap();

                                if let Some(matrix) = renderer.matrix_data_map.get_mut(&index) {
                                    *matrix = MatrixData::new(model_matrix, mesh.matrix);

                                    matrix_data_changes.insert(index);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    for (key, draw) in &renderer.opaque_draws {
        debug_assert_eq!(*key, draw.first_instance);

        if let Some(instance) = renderer.instances.get(key) {
            debug_assert_eq!(*key, instance.matrix_index);
        }
    }

    #[cfg(debug_assertions)]
    for (key, draw) in &renderer.non_opaque_draws {
        debug_assert_eq!(*key, draw.first_instance);

        if let Some(instance) = renderer.instances.get(key) {
            debug_assert_eq!(*key, instance.matrix_index);
        }
    }

    #[cfg(debug_assertions)]
    for key in renderer.opaque_draws.keys() {
        debug_assert!(!renderer.non_opaque_draws.contains_key(key));
    }

    let overlay_instances = mem::take(&mut renderer.overlay_instances);
    for (_, model, _, mesh_index) in &overlay_instances {
        if !renderer
            .animation_matrix_data_map
            .contains_key(&(*model, *mesh_index))
        {
            renderer
                .animation_matrix_data_map
                .insert((*model, *mesh_index), AnimationMatrixData::default());
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
                let index: u32 = index.try_into().unwrap();

                renderer.instances.get_mut(&index).unwrap().color_offset = [0.0; 4];
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
                let index: u32 = index.try_into().unwrap();

                renderer.instances.get_mut(&index).unwrap().color_offset = tint.to_array();
                instances_changes.insert(index);
            }
        }
    }

    let mut instances_changes = instances_changes.into_iter().collect::<Vec<_>>();
    instances_changes.sort();
    let mut matrix_data_changes = matrix_data_changes.into_iter().collect::<Vec<_>>();
    matrix_data_changes.sort();
    let mut draws_changes = draws_changes.into_iter().collect::<Vec<_>>();
    draws_changes.sort();

    let r = renderer.inner_render(
        state.resource_man.clone(),
        state.gui.as_mut().unwrap(),
        camera_pos,
        state.camera.get_matrix(),
        instances_changes,
        matrix_data_changes,
        draws_changes,
        overlay_instances,
        screenshotting,
    );

    gui::reset_custom_paint_state();
    renderer.last_tile_tints = tile_tints;

    r
}

impl Renderer {
    fn inner_render(
        &mut self,
        resource_man: Arc<ResourceManager>,
        gui: &mut Gui,
        camera_pos: Vec3,
        camera_matrix: Matrix4,
        instances_changes: Vec<u32>,
        matrix_data_changes: Vec<u32>,
        draws_changes: Vec<u32>,
        overlay_instances: Vec<OverlayInstance>,
        screenshotting: bool,
    ) -> Result<(), SurfaceError> {
        let size = self.gpu.window.inner_size();

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

        let mut game_staging_belts = [None, None, None];

        {
            if !(self.opaque_draws.is_empty() && self.non_opaque_draws.is_empty()) {
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

                game_staging_belts[2] = gpu::update_indirect_buffer(
                    &mut encoder,
                    &self.gpu.device,
                    &mut self.render_resources.game_resources.indirect_buffer,
                    &draws_changes,
                    &self.opaque_draws,
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
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
                                view: &self.shared_resources.model_depth_texture().1,
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

                    if !self.opaque_draws.is_empty() {
                        let len = *self.opaque_draws.last_entry().unwrap().key() + 1;

                        // Because non-opaque draws leave holes in the opaque draws, we need to do this.
                        let groups = self
                            .non_opaque_draws
                            .keys()
                            .cloned()
                            .chain([len])
                            .scan(0, |state, v| {
                                let prev = *state;
                                *state = v + 1;

                                if prev == v {
                                    Some(None)
                                } else {
                                    Some(Some((prev, v)))
                                }
                            })
                            .flatten()
                            .collect::<Vec<_>>();

                        const BYTE_SIZE: BufferAddress =
                            size_of::<DrawIndexedIndirectArgs>() as BufferAddress;

                        for (start, end) in groups {
                            if start < len && end <= len && start != end {
                                render_pass.multi_draw_indexed_indirect(
                                    &self.render_resources.game_resources.indirect_buffer,
                                    start as BufferAddress * BYTE_SIZE,
                                    end - start,
                                );
                            }
                        }
                    }

                    for draw in self.non_opaque_draws.values() {
                        render_pass.draw_indexed(
                            draw.first_index..(draw.first_index + draw.index_count),
                            draw.base_vertex,
                            draw.first_instance..(draw.first_instance + draw.instance_count),
                        );
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
                        view: &self.shared_resources.model_depth_texture().1,
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
            let surface = SurfaceInfo {
                format: self.gpu.config.format,
                sample_count: 4,
                color_attachments: vec![Some(RenderPassColorAttachment {
                    view: &self.shared_resources.gui_texture().1,
                    resolve_target: Some(&self.shared_resources.gui_texture_resolve().1),
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_format: None,
                depth_attachment: None,
                depth_load_op: None,
            };

            let mut resources = YakuiRenderResources {
                instances: Some(Vec::new()),

                resource_man: resource_man.clone(),
                global_resources: self.global_resources.clone(),
                surface_format: surface.format,
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
                    color_attachments: &surface.color_attachments,
                    depth_stencil_attachment: None,
                    ..Default::default()
                });

                custom_gui_commands = gui.renderer.paint_with::<GameElementPaint>(
                    &mut gui.yak,
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut render_pass,
                    surface,
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

            render_pass.set_pipeline(&self.global_resources.present_pipeline);
            render_pass.set_bind_group(0, self.shared_resources.present_bind_group(), &[]);
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

                render_pass.set_pipeline(&self.global_resources.screenshot_pipeline);
                render_pass.set_bind_group(0, self.shared_resources.screenshot_bind_group(), &[]);
                render_pass.draw(0..3, 0..1);
            }

            let buffer = self.gpu.device.create_buffer(&BufferDescriptor {
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

        self.gpu
            .queue
            .submit([custom_gui_commands, encoder.finish()]);

        for mut belt in game_staging_belts.into_iter().flatten() {
            belt.recall();
        }

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
                    self.screenshot_clipboard
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

        self.gpu.window.pre_present_notify();

        output.present();

        Ok(())
    }
}
