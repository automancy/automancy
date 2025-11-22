use core::{
    cmp::Reverse,
    fmt::{Debug, Display},
    hash::Hash,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use automancy_data::{
    game::coord::TileCoord,
    id::{ModelId, RenderId},
    math::Matrix4,
    rendering::draw::GameDrawInstance,
};
use automancy_game::resources::ResourceManager;

use crate::{
    gpu,
    gpu::{
        GamePipeline, RenderResources,
        data::{GpuDrawInstance, GpuModelMatrixData, GpuWorldMatrixData},
    },
    renderer::model::{GlobalMeshId, ModelManager},
};

mod data {
    use core::{cmp::Reverse, fmt::Debug, hash::Hash};
    use std::collections::BTreeMap;

    use automancy_data::math::{Float, Matrix4};
    use num_traits::{AsPrimitive, NumAssignOps, PrimInt};
    use slice_group_by::GroupBy;

    use crate::{
        gpu::data::{GpuAnimationMatrixData, GpuDrawInstance, GpuModelMatrixData, GpuWorldMatrixData},
        renderer::{
            instance::{InstanceId, InstanceRange},
            model::{GlobalMeshId, ModelManager},
        },
    };

    #[derive(Debug)]
    pub struct AnimationCollection {
        buffer: Vec<GpuAnimationMatrixData>,
        animation_ids: BTreeMap<GlobalMeshId, u16>,
    }

    impl AnimationCollection {
        const NO_ANIMATION: u16 = 0;

        pub fn new() -> Self {
            let mut buffer = Vec::with_capacity(16);
            buffer.insert(Self::NO_ANIMATION as usize, GpuAnimationMatrixData::new(Matrix4::identity()));

            Self {
                buffer,
                animation_ids: Default::default(),
            }
        }

        #[inline]
        pub fn clear(&mut self) {
            self.animation_ids.clear();
            self.buffer.clear();
        }

        #[inline]
        pub fn ensure_animation_exists(&mut self, model_man: &ModelManager, global_id: GlobalMeshId) {
            if model_man.animation_channels_map.contains_key(&global_id) && !self.animation_ids.contains_key(&global_id) {
                let index = self.buffer.len() as u16;
                self.buffer.push(GpuAnimationMatrixData::default());

                self.animation_ids.insert(global_id, index);
            }
        }
        #[inline]
        #[must_use]
        pub fn buffer(&self) -> &Vec<GpuAnimationMatrixData> {
            &self.buffer
        }

        #[inline]
        #[must_use]
        pub fn get(&self, global_id: GlobalMeshId) -> u16 {
            self.animation_ids.get(&global_id).copied().unwrap_or(Self::NO_ANIMATION)
        }

        #[inline]
        pub fn progress_animation(&mut self, model_man: &ModelManager, elapsed: Float) {
            for (&global_id, &index) in &self.animation_ids {
                let animation = &model_man.animation_channels_map[&global_id];
                let wrapped = elapsed % animation.duration;
                debug_assert!(!animation.keyframes.is_empty());

                let anim_pivot = animation.keyframes.partition_point(|(v, ..)| *v < wrapped);
                let matrix = animation.keyframes[anim_pivot - 1].1;

                self.buffer[index as usize] = GpuAnimationMatrixData::new(matrix);
            }
        }
    }

    #[derive(Debug)]
    pub struct InstanceCollection {
        buffer: Vec<GpuDrawInstance>,
        ranges: BTreeMap<GlobalMeshId, InstanceRange>,
        index_map: BTreeMap<InstanceId, u32>,
        should_rebuild: bool,
        to_remove: Vec<u32>,
    }

    impl InstanceCollection {
        pub fn new() -> Self {
            Self {
                buffer: Vec::with_capacity(256),
                ranges: Default::default(),
                index_map: Default::default(),
                should_rebuild: false,
                to_remove: Default::default(),
            }
        }

        #[inline]
        #[must_use]
        pub fn lookup(&mut self, id: InstanceId) -> &mut GpuDrawInstance {
            &mut self.buffer[self.index_map[&id] as usize]
        }

        #[inline]
        #[must_use]
        pub fn buffer(&self) -> &Vec<GpuDrawInstance> {
            &self.buffer
        }

        #[inline]
        #[must_use]
        pub fn buffer_mut(&mut self) -> &mut Vec<GpuDrawInstance> {
            &mut self.buffer
        }

        #[inline]
        #[must_use]
        pub fn ranges(&self) -> impl ExactSizeIterator<Item = (&GlobalMeshId, &InstanceRange)> {
            self.ranges.iter()
        }

        #[inline]
        pub fn add(&mut self, id: InstanceId, instance: GpuDrawInstance) -> u32 {
            self.buffer.push(instance);
            let index = (self.buffer.len() - 1) as u32;

            assert_eq!(
                self.index_map.insert(id, index),
                None,
                "InstanceId should be unique; duplicate InstanceId: {id:?}"
            );
            self.should_rebuild = true;

            index
        }

        #[inline]
        #[must_use]
        pub fn to_flat_instances(&self) -> Vec<(InstanceId, GpuDrawInstance)> {
            let mut flat_buffer = Vec::with_capacity(self.buffer.len());
            for (&id, &index) in &self.index_map {
                flat_buffer.push((id, self.buffer[index as usize]))
            }
            flat_buffer.sort_by_key(|v| v.0.global_id);

            flat_buffer
        }

        #[inline]
        pub fn mark_removal(&mut self, id: InstanceId) {
            self.to_remove.push(self.index_map.remove(&id).unwrap());
        }

        #[inline]
        pub fn flush_removal(&mut self) {
            if self.to_remove.is_empty() {
                return;
            }
            self.to_remove.sort_by(|a, b| a.cmp(b).reverse());

            let mut instance_mapping = BTreeMap::new();
            for &index in &self.to_remove {
                let removed_index = (self.buffer.len() - 1) as u32;
                self.buffer.swap_remove(index as usize);

                if removed_index != index {
                    instance_mapping.insert(removed_index, index);
                }
            }

            for index in self.index_map.values_mut() {
                while let Some(&new_index) = instance_mapping.get(index) {
                    *index = new_index;
                }
            }

            self.to_remove.clear();
            self.should_rebuild = true;
        }

        #[inline]
        pub fn flush(&mut self) {
            if self.should_rebuild {
                let flat_buffer = self.to_flat_instances();

                self.ranges = flat_buffer
                    .binary_group_by_key(|v| v.0.global_id)
                    .scan(0, |acc, group| {
                        let count = group.len() as u32;
                        let start = *acc;
                        *acc += count;

                        Some((group[0].0.global_id, InstanceRange { start, count }))
                    })
                    .collect();

                self.index_map = {
                    // reuse allocation
                    let mut index_map = std::mem::take(&mut self.index_map);
                    index_map.clear();

                    for (index, (id, ..)) in flat_buffer.iter().enumerate() {
                        index_map.insert(*id, index as u32);
                    }

                    index_map
                };

                self.buffer = flat_buffer.into_iter().map(|v| v.1).collect();
                self.should_rebuild = false;
            }
        }

        #[inline]
        pub fn clear(&mut self) {
            self.buffer.clear();
            self.ranges.clear();
            self.index_map.clear();
            self.should_rebuild = false;
            self.to_remove.clear();
        }
    }

    #[derive(Debug, Default)]
    pub struct MatrixCollection<Data, Index> {
        buffer: Vec<Data>,
        index_reuses: BTreeMap<Index, Index>,
    }

    impl<Data, Index> MatrixCollection<Data, Index> {
        #[inline]
        pub fn clear(&mut self) {
            self.buffer.clear();
            self.index_reuses.clear();
        }
    }

    impl<Data, Index> MatrixCollection<Data, Index>
    where
        Data: PartialEq,
        Index: PrimInt + NumAssignOps + AsPrimitive<usize> + Hash + Debug,
        usize: AsPrimitive<Index>,
    {
        #[inline]
        #[must_use]
        pub fn buffer(&self) -> &Vec<Data> {
            &self.buffer
        }

        #[inline]
        pub fn set_all(&mut self, matrix: Data)
        where
            Data: Copy,
        {
            for v in &mut self.buffer {
                *v = matrix;
            }
        }

        #[inline]
        #[must_use]
        pub fn modify_matrix(&mut self, index: Index, matrix: Data) -> Index {
            // TODO support modifying multiple matrices at once, as this is likely and the current method would fragment the matrices in that case

            if self.index_reuses.get_mut(&index).copied().unwrap_or(Index::zero()).is_zero() {
                // the index is *not* reused elsewhere, so let's just modify it
                self.index_reuses.remove(&index);

                *self.buffer.get_mut(index.as_()).unwrap() = matrix;

                index
            } else {
                // the index is reused elsewhere
                let count = self.index_reuses.get_mut(&index).unwrap();

                *count -= Index::one();
                if count.is_zero() {
                    self.index_reuses.remove(&index);
                }

                self.insert_matrix(matrix)
            }
        }

        #[inline]
        #[must_use]
        pub fn insert_matrix(&mut self, matrix: Data) -> Index {
            if Some(&matrix) == self.buffer.last() {
                let index = (self.buffer.len() - 1).as_();

                *self.index_reuses.entry(index).or_insert(Index::zero()) += Index::one();

                index
            } else {
                self.buffer.push(matrix);

                (self.buffer.len() - 1).as_()
            }
        }

        /// Remove an index from the collection by swap removing, and returns the removed index, if a mapping is appropriate (i.e. if the indices are different).
        #[inline]
        #[must_use]
        pub fn remove_matrix(&mut self, index: Index) -> Option<Index> {
            if self.index_reuses.get(&index).copied().unwrap_or(Index::zero()).is_zero() {
                // the index isn't reused elsewhere, let's swap remove it
                self.index_reuses.remove(&index);

                let removed_index = (self.buffer.len() - 1).as_();
                self.buffer.swap_remove(index.as_());

                if removed_index != index {
                    if let Some(reuses) = self.index_reuses.remove(&removed_index) {
                        self.index_reuses.insert(index, reuses);
                    }

                    return Some(removed_index);
                }
            } else if let Some(count) = self.index_reuses.get_mut(&index) {
                // the index is reused elsewhere, let's just decrement the reuse
                *count -= Index::one();

                if count.is_zero() {
                    self.index_reuses.remove(&index);
                }
            }

            None
        }

        /// Remove a list of indices from the collection by swap removing, and returns a mapping of `removed index` -> `new index`.
        ///
        /// `to_remove` needs to be sorted, from largest index to smallest index (reverse order).
        #[inline]
        pub fn remove_matrices(&mut self, to_remove: Vec<Index>, mapping: &mut BTreeMap<Reverse<Index>, Index>) {
            debug_assert!(
                to_remove.is_sorted_by(|a, b| a >= b),
                "to_remove should be sorted from largest to smallest"
            );

            for index in to_remove {
                if let Some(removed_index) = self.remove_matrix(index) {
                    mapping.insert(Reverse(removed_index), index);
                }
            }
        }
    }

    pub type ModelMatrices = MatrixCollection<GpuModelMatrixData, u32>;
    pub type WorldMatrices = MatrixCollection<GpuWorldMatrixData, u16>;
}

pub type DrawId = (TileCoord, RenderId, ModelId);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceId {
    pub global_id: GlobalMeshId,
    pub render_id: RenderId,
    pub coord: TileCoord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstanceRange {
    start: u32,
    count: u32,
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{{{}, {},{}}}", self.coord, self.render_id, self.global_id))
    }
}

#[derive(Debug)]
pub struct DrawInstanceManager {
    to_remove: BTreeSet<DrawId>,

    opaque: data::InstanceCollection,
    non_opaque: data::InstanceCollection,

    model_matrices: data::ModelMatrices,
    world_matrices: data::WorldMatrices,

    animations: data::AnimationCollection,
}

impl Default for DrawInstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawInstanceManager {
    pub fn new() -> Self {
        Self {
            to_remove: Default::default(),

            opaque: data::InstanceCollection::new(),
            non_opaque: data::InstanceCollection::new(),

            model_matrices: Default::default(),
            world_matrices: Default::default(),

            animations: data::AnimationCollection::new(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.to_remove.clear();

        self.opaque.clear();
        self.non_opaque.clear();

        self.model_matrices.clear();
        self.world_matrices.clear();

        self.animations.clear();
    }
}

impl DrawInstanceManager {
    #[cfg(debug_assertions)]
    pub fn get_game_instances(&self) -> Vec<(InstanceId, GameDrawInstance)> {
        #[inline]
        fn collect(
            result: &mut Vec<(InstanceId, GameDrawInstance)>,
            instances: &data::InstanceCollection,
            model_matrices: &data::ModelMatrices,
            world_matrices: &data::WorldMatrices,
        ) {
            let flat_instances = instances.to_flat_instances();

            for (id, instance) in flat_instances {
                use automancy_data::rendering::colors::Rgba;

                let model_matrix = model_matrices.buffer()[instance.model_matrix_index as usize];
                let world_matrix = world_matrices.buffer()[instance.world_matrix_index as usize];

                result.push((
                    id,
                    GameDrawInstance {
                        color_offset: Rgba::from_slice(&instance.color_offset),
                        alpha: instance.alpha,
                        model_matrix: Matrix4::from_col_arrays(model_matrix.model_matrix),
                        world_matrix: Matrix4::from_col_arrays(world_matrix.world_matrix),
                    },
                ));
            }
        }

        let mut result = Vec::new();
        collect(&mut result, &self.opaque, &self.model_matrices, &self.world_matrices);
        collect(&mut result, &self.non_opaque, &self.model_matrices, &self.world_matrices);

        result.sort_by_key(|v| v.0.coord);
        result.sort_by_key(|v| v.0.global_id);

        result
    }

    #[inline]
    pub fn modify_instances(
        &mut self,
        resource_man: &ResourceManager,
        model_man: &ModelManager,
        (coord, render_id, model_id): DrawId,
        mut f: impl FnMut(InstanceId, &mut GpuDrawInstance),
    ) {
        for &global_id in model_man.global_model_mesh_ids(model_id, ModelId(resource_man.registry.model_ids.tile_missing)) {
            let id = InstanceId { global_id, render_id, coord };

            let instances = if model_man.mesh_metadata[&global_id].opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            let instance = instances.lookup(id);

            f(id, instance)
        }
    }

    #[inline]
    pub fn set_matrix(
        &mut self,
        resource_man: &ResourceManager,
        model_man: &ModelManager,
        (coord, render_id, model_id): DrawId,
        (model_matrix, world_matrix): (Option<Matrix4>, Option<Matrix4>),
    ) {
        for &global_id in model_man.global_model_mesh_ids(model_id, ModelId(resource_man.registry.model_ids.tile_missing)) {
            let id = InstanceId { global_id, render_id, coord };

            let instances = if model_man.mesh_metadata[&global_id].opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            let instance = instances.lookup(id);

            if let Some(matrix) = model_matrix {
                instance.model_matrix_index = self
                    .model_matrices
                    .modify_matrix(instance.model_matrix_index, GpuModelMatrixData::new(matrix));
            }

            if let Some(matrix) = world_matrix {
                instance.world_matrix_index = self
                    .world_matrices
                    .modify_matrix(instance.world_matrix_index, GpuWorldMatrixData::new(matrix));
            }
        }
    }

    #[inline]
    pub fn set_all_world_matrix(&mut self, matrix: Matrix4) {
        let matrix = GpuWorldMatrixData::new(matrix);
        self.world_matrices.set_all(matrix);
    }
}

impl DrawInstanceManager {
    #[inline]
    pub fn insert(
        &mut self,
        resource_man: &ResourceManager,
        model_man: &ModelManager,
        (coord, render_id, model_id): DrawId,
        GameDrawInstance {
            color_offset,
            alpha,
            model_matrix,
            world_matrix,
        }: GameDrawInstance,
    ) {
        for &global_id in model_man.global_model_mesh_ids(model_id, ModelId(resource_man.registry.model_ids.tile_missing)) {
            let id = InstanceId { global_id, render_id, coord };

            let instances = if model_man.mesh_metadata[&global_id].opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            self.animations.ensure_animation_exists(model_man, global_id);

            instances.add(
                id,
                GpuDrawInstance::new(
                    color_offset,
                    alpha,
                    self.model_matrices.insert_matrix(GpuModelMatrixData::new(model_matrix)),
                    self.world_matrices.insert_matrix(GpuWorldMatrixData::new(world_matrix)),
                    self.animations.get(global_id),
                ),
            );
        }
    }

    #[inline]
    pub fn remove(&mut self, id: DrawId) {
        self.to_remove.insert(id);
    }

    #[inline]
    pub fn flush(&mut self, resource_man: &ResourceManager, model_man: &ModelManager) {
        if !self.to_remove.is_empty() {
            self.opaque.flush();
            self.non_opaque.flush();

            let mut model_matrix_to_remove = Vec::new();
            let mut world_matrix_to_remove = Vec::new();

            // reuse allocation
            for &(coord, render_id, model_id) in &self.to_remove {
                for &global_id in model_man.global_model_mesh_ids(model_id, ModelId(resource_man.registry.model_ids.tile_missing)) {
                    let id = InstanceId { global_id, render_id, coord };

                    let is_opaque = model_man.mesh_metadata[&global_id].opaque;
                    let instances = if is_opaque { &mut self.opaque } else { &mut self.non_opaque };

                    let instance = instances.lookup(id);
                    model_matrix_to_remove.push(instance.model_matrix_index);
                    world_matrix_to_remove.push(instance.world_matrix_index);

                    instances.mark_removal(id);
                }
            }
            self.to_remove.clear();

            model_matrix_to_remove.sort_by(|a, b| a.cmp(b).reverse());
            world_matrix_to_remove.sort_by(|a, b| a.cmp(b).reverse());

            let mut model_matrix_mapping = BTreeMap::new();
            let mut world_matrix_mapping = BTreeMap::new();
            self.model_matrices.remove_matrices(model_matrix_to_remove, &mut model_matrix_mapping);
            self.world_matrices.remove_matrices(world_matrix_to_remove, &mut world_matrix_mapping);

            for instance in self.opaque.buffer_mut().iter_mut().chain(self.non_opaque.buffer_mut()) {
                while let Some(&new_index) = model_matrix_mapping.get(&Reverse(instance.model_matrix_index)) {
                    instance.model_matrix_index = new_index;
                }

                while let Some(&new_index) = world_matrix_mapping.get(&Reverse(instance.world_matrix_index)) {
                    instance.world_matrix_index = new_index;
                }
            }

            self.opaque.flush_removal();
            self.non_opaque.flush_removal();
        }

        self.opaque.flush();
        self.non_opaque.flush();
    }

    #[inline]
    pub fn upload_animation(&mut self, res: &mut RenderResources, start_instant: Instant, model_man: &ModelManager) {
        let elapsed = start_instant.elapsed().as_secs_f32();
        self.animations.progress_animation(model_man, elapsed);

        if std::mem::size_of_val(self.animations.buffer().as_slice()) as u64 > res.main_game_res.game_pipeline.animation_matrix_data_buffer.size() {
            res.main_game_res.game_pipeline = GamePipeline::new(
                &res.device,
                &res.global_res,
                res.main_game_res.game_pipeline.model_matrix_data_buffer.size(),
                res.main_game_res.game_pipeline.world_matrix_data_buffer.size(),
                util::new_size(
                    res.main_game_res.game_pipeline.animation_matrix_data_buffer.size(),
                    std::mem::size_of_val(self.animations.buffer().as_slice()),
                ),
            );

            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.model_matrix_data_buffer,
                self.model_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.world_matrix_data_buffer,
                self.world_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.animation_matrix_data_buffer,
                self.animations.buffer(),
            );
        } else {
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.animation_matrix_data_buffer,
                self.animations.buffer(),
            );
        }
    }

    #[inline]
    #[must_use]
    pub fn collect_draw_calls(&mut self, res: &mut RenderResources, model_man: &ModelManager) -> [Vec<wgpu::util::DrawIndexedIndirectArgs>; 2] {
        if std::mem::size_of_val(self.model_matrices.buffer().as_slice()) as u64 > res.main_game_res.game_pipeline.model_matrix_data_buffer.size() {
            // TODO allow reusing buffers that don't need resizing
            res.main_game_res.game_pipeline = GamePipeline::new(
                &res.device,
                &res.global_res,
                util::new_size(
                    res.main_game_res.game_pipeline.model_matrix_data_buffer.size(),
                    std::mem::size_of_val(self.model_matrices.buffer().as_slice()),
                ),
                res.main_game_res.game_pipeline.world_matrix_data_buffer.size(),
                res.main_game_res.game_pipeline.animation_matrix_data_buffer.size(),
            );

            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.model_matrix_data_buffer,
                self.model_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.world_matrix_data_buffer,
                self.world_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.animation_matrix_data_buffer,
                self.animations.buffer(),
            );
        } else {
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.model_matrix_data_buffer,
                self.model_matrices.buffer(),
            );
        }

        if std::mem::size_of_val(self.world_matrices.buffer().as_slice()) as u64 > res.main_game_res.game_pipeline.world_matrix_data_buffer.size() {
            res.main_game_res.game_pipeline = GamePipeline::new(
                &res.device,
                &res.global_res,
                res.main_game_res.game_pipeline.model_matrix_data_buffer.size(),
                util::new_size(
                    res.main_game_res.game_pipeline.world_matrix_data_buffer.size(),
                    std::mem::size_of_val(self.world_matrices.buffer().as_slice()),
                ),
                res.main_game_res.game_pipeline.animation_matrix_data_buffer.size(),
            );

            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.model_matrix_data_buffer,
                self.model_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.world_matrix_data_buffer,
                self.world_matrices.buffer(),
            );
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.animation_matrix_data_buffer,
                self.animations.buffer(),
            );
        } else {
            gpu::util::upload_buffer(
                &res.queue,
                &res.main_game_res.game_pipeline.world_matrix_data_buffer,
                self.world_matrices.buffer(),
            );
        }

        gpu::util::upload_buffer_or_recreate(
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.opaque_instance_buffer,
            self.opaque.buffer(),
        );
        gpu::util::upload_buffer_or_recreate(
            &res.device,
            &res.queue,
            &mut res.main_game_res.game_pipeline.non_opaque_instance_buffer,
            self.non_opaque.buffer(),
        );

        #[inline]
        fn collect<'a, I: IntoIterator<Item = (&'a GlobalMeshId, &'a InstanceRange)>>(
            model_man: &ModelManager,
            instance_ranges: I,
        ) -> Vec<wgpu::util::DrawIndexedIndirectArgs> {
            let mut draws = Vec::with_capacity(256);

            for (&global_id, &range) in instance_ranges {
                let index_range = model_man.index_ranges[&global_id];

                draws.push(wgpu::util::DrawIndexedIndirectArgs {
                    index_count: index_range.count,
                    first_index: index_range.pos,
                    base_vertex: index_range.base_vertex,
                    instance_count: range.count,
                    first_instance: range.start,
                });
            }

            draws
        }

        [collect(model_man, self.opaque.ranges()), collect(model_man, self.non_opaque.ranges())]
    }
}

mod util {
    #[inline]
    pub(crate) fn new_size(gpu_size: u64, cpu_size: usize) -> u64 {
        let gpu_size = gpu_size * 2;
        let cpu_size = cpu_size as u64;

        if gpu_size < cpu_size { cpu_size } else { gpu_size }
    }
}
