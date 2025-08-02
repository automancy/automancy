use std::collections::BTreeMap;

use automancy_data::{
    coord::TileCoord,
    id::{ModelId, RenderId},
    rendering::{GpuGameMatrixData, GpuInstance, GpuWorldMatrixData, MeshId},
};
use automancy_resources::{ResourceManager, rhai_render::RenderCommand};
use hashbrown::{HashMap, HashSet};
use range_set_blaze::RangeSetBlaze;

use crate::rendering::game::AnimationCache;

type DrawId = (RenderId, ModelId, MeshId);

pub type AnimationCache = HashMap<ModelId, HashMap<usize, Matrix4>>;
pub type AnimationMatrixDataMap = OrderMap<(ModelId, usize), GpuAnimationMatrix>;

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

pub struct DrawStorage {
    instances: Vec<GpuInstance>,
    matrices: Vec<GpuGameMatrixData>,
    world_matrices: Vec<GpuWorldMatrixData>,
    animation_matrices: AnimationCache,

    object_ids: Vec<DrawId>,
    coord_to_index: HashMap<TileCoord, DrawId>,
    instance_ranges: BTreeMap<(ModelId, usize), RangeSetBlaze<usize>>,
}

impl DrawStorage {
    pub fn consume_commands(
        resource_man: &ResourceManager,
        commands: HashMap<TileCoord, Vec<RenderCommand>>,
    ) {
        let mut instances_changes = HashSet::new();
        let mut matrix_data_changes = HashSet::new();

        for batch in commands {
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
                let (model, (meshes, ..)) = resource_man.mesh_or_missing_tile_mesh(&model);

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
                        renderer.matrix_data_map[index] = GpuGameMatrixData::default();
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
                                *matrix = GpuGameMatrixData::new(model_matrix, mesh.matrix);

                                matrix_data_changes.insert(index);
                            }
                        }
                    }
                }
            }
        }
    }
}
