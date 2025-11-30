use core::{
    fmt::{Debug, Display},
    hash::Hash,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use automancy_data::{id::ModelId, math::Matrix4, rendering::draw::GameDrawInstance};

use crate::{
    ModelManager, gpu,
    instance::data::{ModelMatrixIndex, WorldMatrixIndex},
};

const CPU_INSTANCE_ALIGNMENT: u32 = 64;
const GPU_INSTANCE_SIZE: u64 = std::mem::size_of::<gpu::data::GpuGameDrawInstance>() as u64;

#[inline(always)]
fn cpu_instance_size(size: u32) -> u32 {
    gpu::util::align_u32::<{ CPU_INSTANCE_ALIGNMENT }>(size)
}

mod data {
    use core::fmt::Debug;
    use std::{
        collections::{BTreeMap, BTreeSet},
        time::Instant,
    };

    use automancy_data::{math::Matrix4, rendering::draw::GameDrawInstance};

    use crate::{GameInstanceId, InstanceIndex, MeshId, ModelManager, gpu};

    #[derive(Debug, Clone)]
    pub struct AnimationCollection {
        pub(super) buffer: Vec<gpu::data::GpuGameAnimationMatrixData>,
        animation_ids: BTreeMap<MeshId, u16>,
    }

    impl Default for AnimationCollection {
        fn default() -> Self {
            Self {
                animation_ids: Default::default(),
                buffer: vec![gpu::data::GpuGameAnimationMatrixData::default()],
            }
        }
    }

    #[cfg_attr(feature = "profile", profiling::all_functions)]
    impl AnimationCollection {
        const NO_ANIMATION: u16 = 0;

        #[inline]
        pub(crate) fn clear(&mut self) {
            self.animation_ids.clear();
            self.buffer.clear();
            self.buffer.push(gpu::data::GpuGameAnimationMatrixData::default());
        }

        #[inline]
        pub(crate) fn ensure_animation_exists(&mut self, model_man: &ModelManager, mesh_id: MeshId) {
            if !model_man.get_animation(&mesh_id).is_empty() && !self.animation_ids.contains_key(&mesh_id) {
                let index = self.buffer.len() as u16;
                self.buffer.push(gpu::data::GpuGameAnimationMatrixData::default());

                self.animation_ids.insert(mesh_id, index);
            }
        }

        #[inline]
        #[must_use]
        pub(crate) fn get(&self, mesh_id: MeshId) -> u16 {
            self.animation_ids.get(&mesh_id).copied().unwrap_or(Self::NO_ANIMATION)
        }

        #[inline]
        pub(crate) fn progress_animation(&mut self, model_man: &mut ModelManager, frame_start: Instant) {
            for (&mesh_id, &index) in &self.animation_ids {
                debug_assert_ne!(index, Self::NO_ANIMATION);

                let matrix = model_man.sample_animation(&mesh_id, frame_start);
                self.buffer[index as usize] = gpu::data::GpuGameAnimationMatrixData::new(matrix);
            }
        }

        #[inline]
        pub fn upload(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
            #[cfg(feature = "profile")]
            profiling::scope!("upload_animation_matrices");
            gpu::util::upload_buffer(queue, buffer, 0, bytemuck::cast_slice(&self.buffer));
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct MatrixCollection<Data, Index> {
        pub(super) buffer: Vec<Data>,
        index_reuses: BTreeMap<Index, Index>,
        pub(super) modified: bool,
    }

    #[cfg_attr(feature = "profile", profiling::all_functions)]
    impl<Data, Index> MatrixCollection<Data, Index> {
        #[inline]
        pub fn clear(&mut self) {
            self.buffer.clear();
            self.index_reuses.clear();
            self.modified = true;
        }

        #[inline]
        pub fn set_all(&mut self, matrix: Data)
        where
            Data: Copy,
        {
            self.modified = true;

            for v in &mut self.buffer {
                *v = matrix;
            }
        }
    }

    macro_rules! impl_matrix_collection {
        ($Index:ty) => {
            #[cfg_attr(feature = "profile", profiling::all_functions)]
            #[allow(unused)]
            impl<Data: PartialEq + Copy> MatrixCollection<Data, $Index> {
                #[inline]
                #[must_use]
                pub fn modify_matrix(&mut self, index: $Index, matrix: Data) -> $Index {
                    // TODO support modifying multiple matrices at once, as this is likely and the current method would fragment the matrices in that case
                    self.modified = true;

                    if self.index_reuses.get_mut(&index).copied().unwrap_or(0) == 0 {
                        // the index is *not* reused elsewhere, so let's just modify it
                        self.index_reuses.remove(&index);

                        *self.buffer.get_mut(index as usize).expect("matrix should be present") = matrix;

                        index
                    } else {
                        // the index is reused elsewhere
                        let count = self.index_reuses.get_mut(&index).expect("matrix should be present");

                        *count -= 1;
                        if *count == 0 {
                            self.index_reuses.remove(&index);
                        }

                        self.insert_matrix(matrix)
                    }
                }

                #[inline]
                #[must_use]
                pub fn insert_matrix(&mut self, matrix: Data) -> $Index {
                    if Some(&matrix) == self.buffer.last() {
                        let index = (self.buffer.len() - 1) as $Index;

                        *self.index_reuses.entry(index).or_insert(0) += 1;

                        index
                    } else if Some(&matrix) == self.buffer.first() {
                        let index = 0 as $Index;

                        *self.index_reuses.entry(index).or_insert(0) += 1;

                        index
                    } else {
                        self.modified = true;

                        self.buffer.push(matrix);

                        (self.buffer.len() - 1) as $Index
                    }
                }

                /// Remove an index from the collection by swap removing, and returns the removed index, if a mapping is appropriate (i.e. if the indices are different).
                #[inline]
                #[must_use]
                pub fn remove_matrix(&mut self, index: $Index) -> Option<$Index> {
                    if self.index_reuses.get(&index).copied().unwrap_or(0) == 0 {
                        self.modified = true;

                        // the index isn't reused elsewhere, let's swap remove it
                        self.index_reuses.remove(&index);

                        let removed_index = (self.buffer.len() - 1) as $Index;
                        self.buffer.swap_remove(index as usize);

                        if removed_index != index {
                            if let Some(reuses) = self.index_reuses.remove(&removed_index) {
                                self.index_reuses.insert(index, reuses);
                            }

                            return Some(removed_index);
                        }
                    } else {
                        let count = self.index_reuses.get_mut(&index).expect("matrix should be present");

                        // the index is reused elsewhere, let's just decrement the reuse
                        *count -= 1;

                        if *count == 0 {
                            self.index_reuses.remove(&index);
                        }
                    }

                    None
                }

                /// Remove a list of indices from the collection by swap removing, and returns a mapping of `removed index` -> `new index`.
                ///
                /// Note: `to_remove` needs to be sorted.
                #[inline]
                pub fn remove_matrices<'a>(
                    &mut self,
                    to_remove: impl DoubleEndedIterator<Item = &'a $Index>,
                    remapping: &mut BTreeMap<$Index, $Index>,
                ) {
                    for &index in to_remove.rev() {
                        if let Some(removed_index) = self.remove_matrix(index) {
                            remapping.insert(removed_index, index);
                        }
                    }
                }
            }
        };

        ($($Index: ty),+) => {
            $( impl_matrix_collection!($Index); )*
        };
    }

    impl_matrix_collection!(u8, u16, u32, u64, usize);

    pub type ModelMatrixIndex = u32;
    pub type WorldMatrixIndex = u16;
    pub type ModelMatrices = MatrixCollection<gpu::data::GpuGameModelMatrixData, ModelMatrixIndex>;
    pub type WorldMatrices = MatrixCollection<gpu::data::GpuGameWorldMatrixData, WorldMatrixIndex>;

    #[derive(Debug)]
    pub struct GameInstancedDrawCalls<'a> {
        pub buffer: &'a [wgpu::wgt::DrawIndexedIndirectArgs],
        pub modified: bool,
    }

    #[derive(Debug)]
    pub struct GameInstancedDrawCallsCollection<'a> {
        pub opaque: GameInstancedDrawCalls<'a>,
        pub non_opaque: GameInstancedDrawCalls<'a>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum InstancedMeshIndex {
        Staging(u32),
        Staged(u32),
    }

    #[derive(Debug, Clone)]
    pub struct InstanceCollection<Index: InstanceIndex> {
        label: &'static str,
        instance_man_label: &'static str,

        draws: Vec<wgpu::util::DrawIndexedIndirectArgs>,
        draws_rebuilt: bool,

        pub(super) staging_buffer: Vec<(GameInstanceId<Index>, gpu::data::GpuGameDrawInstance)>,
        pub(super) id_buffer: Vec<GameInstanceId<Index>>,
        pub(super) buffer: Vec<gpu::data::GpuGameDrawInstance>,
        instance_ranges: BTreeMap<MeshId, super::InstanceRange>,
        mesh_map: BTreeMap<GameInstanceId<Index>, BTreeMap<MeshId, InstancedMeshIndex>>,
        to_remove: BTreeSet<InstancedMeshIndex>,

        instance_remapping: BTreeMap<InstancedMeshIndex, InstancedMeshIndex>,

        pub(super) should_rebuild_buf: bool,
        pub(super) instance_offset: u32,
        pub(super) modified: bool,
    }

    #[cfg_attr(feature = "profile", profiling::all_functions)]
    impl<Index: InstanceIndex> InstanceCollection<Index> {
        pub fn new(label: &'static str, instance_man_label: &'static str) -> Self {
            Self {
                label,
                instance_man_label,

                draws: Vec::with_capacity(64),
                draws_rebuilt: false,

                staging_buffer: Vec::with_capacity(super::CPU_INSTANCE_ALIGNMENT as usize * 8),
                id_buffer: Vec::with_capacity(super::CPU_INSTANCE_ALIGNMENT as usize * 4),
                buffer: Vec::with_capacity(super::CPU_INSTANCE_ALIGNMENT as usize * 4),
                instance_ranges: Default::default(),
                mesh_map: Default::default(),
                to_remove: Default::default(),

                instance_remapping: BTreeMap::default(),

                should_rebuild_buf: false,
                instance_offset: 0,
                modified: false,
            }
        }

        #[inline]
        pub(super) fn add(&mut self, id: GameInstanceId<Index>, data: gpu::data::GpuGameDrawInstance) -> u32 {
            self.staging_buffer.push((id, data));
            let index = (self.staging_buffer.len() - 1) as u32;

            self.mesh_map
                .entry(id)
                .or_default()
                .insert(data.mesh_id, InstancedMeshIndex::Staging(index));

            self.should_rebuild_buf = true;

            index
        }

        #[inline]
        #[must_use]
        pub(super) fn lookup(&mut self, id: GameInstanceId<Index>, mesh_id: MeshId) -> &mut gpu::data::GpuGameDrawInstance {
            let index = self.mesh_map[&id][&mesh_id];

            match index {
                InstancedMeshIndex::Staging(index) => &mut self.staging_buffer[index as usize].1,
                InstancedMeshIndex::Staged(index) => &mut self.buffer[index as usize],
            }
        }

        #[inline]
        #[must_use]
        pub(super) fn cpu_buffer_len(&self) -> u32 {
            super::cpu_instance_size(self.buffer.len() as u32)
        }

        #[inline]
        #[must_use]
        pub(super) fn gpu_buffer_len(&self) -> u64 {
            self.cpu_buffer_len() as u64 * super::GPU_INSTANCE_SIZE
        }

        #[inline]
        #[must_use]
        pub(super) fn get_draws(&mut self) -> GameInstancedDrawCalls<'_> {
            if self.draws_rebuilt {
                self.draws_rebuilt = false;

                GameInstancedDrawCalls {
                    buffer: &self.draws,
                    modified: true,
                }
            } else {
                GameInstancedDrawCalls {
                    buffer: &self.draws,
                    modified: false,
                }
            }
        }

        #[inline]
        pub(super) fn remap_indices(&mut self, model_matrix_remapping: &BTreeMap<u32, u32>, world_matrix_remapping: &BTreeMap<u16, u16>) {
            for instance in self
                .buffer
                .iter_mut()
                .chain(self.staging_buffer.iter_mut().map(|(_, instance)| instance))
            {
                while let Some(&new_index) = model_matrix_remapping.get(&instance.model_matrix_index) {
                    instance.model_matrix_index = new_index;
                }

                while let Some(&new_index) = world_matrix_remapping.get(&instance.world_matrix_index) {
                    instance.world_matrix_index = new_index;
                }
            }

            self.modified = true;
        }

        pub(super) fn flush_removal(&mut self) {
            if self.to_remove.is_empty() {
                return;
            }
            log::debug!(
                "[{}] Flushing instance removals for collection <{}>.",
                self.instance_man_label,
                self.label
            );

            self.instance_remapping.clear();

            for &index in self.to_remove.iter().rev() {
                let removed_index = match index {
                    InstancedMeshIndex::Staging(index) => {
                        let r = InstancedMeshIndex::Staging((self.staging_buffer.len() - 1) as u32);
                        self.staging_buffer.swap_remove(index as usize);
                        r
                    },
                    InstancedMeshIndex::Staged(index) => {
                        let r = InstancedMeshIndex::Staged((self.buffer.len() - 1) as u32);
                        self.buffer.swap_remove(index as usize);
                        self.id_buffer.swap_remove(index as usize);
                        r
                    },
                };

                if removed_index != index {
                    self.instance_remapping.insert(removed_index, index);
                }
            }

            for index_map in self.mesh_map.values_mut() {
                for index in index_map.values_mut() {
                    while let Some(&new_index) = self.instance_remapping.get(index) {
                        *index = new_index;
                    }
                }
            }

            self.to_remove.clear();
            self.should_rebuild_buf = true;
        }

        pub(super) fn rebuild(&mut self, model_man: &ModelManager) {
            if self.should_rebuild_buf {
                self.modified = true;

                log::debug!(
                    "[{}] Rebuilding instance mapping for collection <{}>.",
                    self.instance_man_label,
                    self.label
                );

                self.mesh_map.clear();
                self.instance_ranges.clear();

                self.staging_buffer
                    .extend(self.id_buffer.iter().copied().zip(self.buffer.iter().copied()));
                self.staging_buffer.sort_by_key(|(_, instance)| instance.mesh_id);

                self.id_buffer.clear();
                self.buffer.clear();
                for (index, &(id, instance)) in self.staging_buffer.iter().enumerate() {
                    self.id_buffer.push(id);
                    self.buffer.push(instance);

                    self.mesh_map
                        .entry(id)
                        .or_default()
                        .insert(instance.mesh_id, InstancedMeshIndex::Staged(index as u32));
                    self.instance_ranges
                        .entry(instance.mesh_id)
                        .or_insert(super::InstanceRange {
                            start: index as _,
                            count: 0,
                        })
                        .count += 1;
                }
                self.staging_buffer.clear();

                self.draws.clear();
                for (&mesh_id, &instance_range) in &self.instance_ranges {
                    let index_range = model_man.index_ranges[&mesh_id];

                    self.draws.push(wgpu::util::DrawIndexedIndirectArgs {
                        index_count: index_range.count,
                        first_index: index_range.pos,
                        base_vertex: index_range.base_vertex,
                        instance_count: instance_range.count,
                        first_instance: self.instance_offset + instance_range.start,
                    });
                }

                self.should_rebuild_buf = false;
                self.draws_rebuilt = true;
            }
        }

        #[inline]
        pub fn mark_removal(&mut self, id: GameInstanceId<Index>, mesh_id: MeshId) {
            self.to_remove.insert(
                self.mesh_map
                    .get_mut(&id)
                    .expect("model should be present")
                    .remove(&mesh_id)
                    .expect("mesh should be present"),
            );
        }

        #[inline]
        pub fn mark_dirty(&mut self) {
            self.modified = true;
            self.should_rebuild_buf = true;
        }

        #[inline]
        pub fn clear(&mut self) {
            self.draws.clear();
            self.draws_rebuilt = false;

            self.staging_buffer.clear();
            self.id_buffer.clear();
            self.buffer.clear();
            self.instance_ranges.clear();
            self.mesh_map.clear();
            self.to_remove.clear();

            self.instance_remapping.clear();

            self.should_rebuild_buf = true;
            self.modified = true;
        }

        #[inline]
        pub fn to_game_draw_instances(
            &self,
            model_man: &ModelManager,
            model_matrices: &ModelMatrices,
            world_matrices: &WorldMatrices,
            inserted_ids: &mut BTreeSet<super::GameInstanceId<Index>>,
        ) -> Vec<(super::GameInstanceId<Index>, GameDrawInstance)> {
            let mut buffer = Vec::with_capacity(self.buffer.capacity());

            for (&id, index_map) in &self.mesh_map {
                for (&mesh_id, &index) in index_map {
                    let model_id = model_man.meshes[&mesh_id].model_id;
                    let id = super::GameInstanceId {
                        model_id,
                        index: id.index,
                    };
                    if inserted_ids.contains(&id) {
                        continue;
                    }

                    let instance = match index {
                        InstancedMeshIndex::Staging(index) => self.staging_buffer[index as usize].1,
                        InstancedMeshIndex::Staged(index) => self.buffer[index as usize],
                    };

                    let model_matrix = model_matrices.buffer[instance.model_matrix_index as usize];
                    let world_matrix = world_matrices.buffer[instance.world_matrix_index as usize];

                    let model_matrix = Matrix4::from_col_arrays(model_matrix.model_matrix);
                    let world_matrix = Matrix4::from_col_arrays(world_matrix.world_matrix);

                    buffer.push((
                        id,
                        GameDrawInstance {
                            color_offset: instance.color_offset,
                            alpha: instance.alpha,
                            model_matrix,
                            world_matrix,
                        },
                    ));
                    inserted_ids.insert(id);
                }
            }

            buffer
        }
    }
}

pub trait InstanceIndex: Debug + Copy + Hash + Ord + Eq {}

impl<T: Debug + Copy + Hash + Ord + Eq> InstanceIndex for T {}

/// Uniquely identifies a "Draw Instance" on the CPU.
///
/// An instance is uniquely disambiguated by two things:
/// - The [`ModelId`], which uniquely identifies a model in the [`ResourceManager`].
/// - The `Index`, which is a generic type that disambiguates the models. It must implement [`InstanceIndex`].
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameInstanceId<Index: InstanceIndex> {
    pub model_id: ModelId,
    pub index: Index,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstanceRange {
    start: u32,
    count: u32,
}

impl<T: Display + InstanceIndex> Display for GameInstanceId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{{{} ({})}}", self.index, self.model_id))
    }
}

#[derive(Debug, Clone)]
pub struct GameInstanceManager<Index: InstanceIndex> {
    #[allow(unused)]
    label: &'static str,

    to_remove: BTreeSet<GameInstanceId<Index>>,

    opaque: data::InstanceCollection<Index>,
    non_opaque: data::InstanceCollection<Index>,

    model_matrices: data::ModelMatrices,
    world_matrices: data::WorldMatrices,

    model_matrix_to_remove: BTreeSet<ModelMatrixIndex>,
    world_matrix_to_remove: BTreeSet<WorldMatrixIndex>,
    model_matrix_remapping: BTreeMap<ModelMatrixIndex, ModelMatrixIndex>,
    world_matrix_remapping: BTreeMap<WorldMatrixIndex, WorldMatrixIndex>,

    pub animations: data::AnimationCollection,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl<Index: InstanceIndex> GameInstanceManager<Index> {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,

            to_remove: Default::default(),

            opaque: data::InstanceCollection::new("opaque", label),
            non_opaque: data::InstanceCollection::new("non_opaque", label),

            model_matrices: Default::default(),
            world_matrices: Default::default(),

            model_matrix_to_remove: Default::default(),
            world_matrix_to_remove: Default::default(),
            model_matrix_remapping: Default::default(),
            world_matrix_remapping: Default::default(),

            animations: Default::default(),
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

        self.mark_dirty();
    }

    #[inline]
    pub fn modify_instances(
        &mut self,
        model_man: &ModelManager,
        id: GameInstanceId<Index>,
        mut f: impl FnMut(GameInstanceId<Index>, &mut gpu::data::GpuGameDrawInstance),
    ) {
        for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
            let instances = if model_man.meshes[&mesh_id].metadata.opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };
            instances.modified = true;

            let instance = instances.lookup(id, mesh_id);
            f(id, instance);
        }
    }

    #[inline]
    pub fn set_matrix(
        &mut self,
        model_man: &ModelManager,
        id: GameInstanceId<Index>,
        (model_matrix, world_matrix): (Option<Matrix4>, Option<Matrix4>),
    ) {
        for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
            let instances = if model_man.meshes[&mesh_id].metadata.opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            if let Some(matrix) = model_matrix {
                let instance = instances.lookup(id, mesh_id);

                let matrix = gpu::data::GpuGameModelMatrixData::new(matrix);
                if self.model_matrices.buffer[instance.model_matrix_index as usize] != matrix {
                    instance.model_matrix_index = self.model_matrices.modify_matrix(instance.model_matrix_index, matrix);
                    instances.modified = true;
                }
            }

            if let Some(matrix) = world_matrix {
                let instance = instances.lookup(id, mesh_id);

                let matrix = gpu::data::GpuGameWorldMatrixData::new(matrix);
                if self.world_matrices.buffer[instance.world_matrix_index as usize] != matrix {
                    instance.world_matrix_index = self.world_matrices.modify_matrix(instance.world_matrix_index, matrix);
                    instances.modified = true;
                }
            }
        }
    }

    #[inline]
    pub fn mul_matrix_left(
        &mut self,
        model_man: &ModelManager,
        id: GameInstanceId<Index>,
        (model_matrix, world_matrix): (Option<Matrix4>, Option<Matrix4>),
    ) {
        for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
            let instances = if model_man.meshes[&mesh_id].metadata.opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            if let Some(matrix) = model_matrix {
                let instance = instances.lookup(id, mesh_id);

                let old_matrix = self.model_matrices.buffer[instance.model_matrix_index as usize];
                instance.model_matrix_index = self.model_matrices.modify_matrix(
                    instance.model_matrix_index,
                    gpu::data::GpuGameModelMatrixData::new(matrix * Matrix4::from_col_arrays(old_matrix.model_matrix)),
                );
                instances.modified = true;
            }

            if let Some(matrix) = world_matrix {
                let instance = instances.lookup(id, mesh_id);

                let old_matrix = self.world_matrices.buffer[instance.world_matrix_index as usize];
                instance.world_matrix_index = self.world_matrices.modify_matrix(
                    instance.world_matrix_index,
                    gpu::data::GpuGameWorldMatrixData::new(matrix * Matrix4::from_col_arrays(old_matrix.world_matrix)),
                );
                instances.modified = true;
            }
        }
    }

    #[inline]
    pub fn mul_matrix_right(
        &mut self,
        model_man: &ModelManager,
        id: GameInstanceId<Index>,
        (model_matrix, world_matrix): (Option<Matrix4>, Option<Matrix4>),
    ) {
        for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
            let instances = if model_man.meshes[&mesh_id].metadata.opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            if let Some(matrix) = model_matrix {
                let instance = instances.lookup(id, mesh_id);

                let old_matrix = self.model_matrices.buffer[instance.model_matrix_index as usize];
                instance.model_matrix_index = self.model_matrices.modify_matrix(
                    instance.model_matrix_index,
                    gpu::data::GpuGameModelMatrixData::new(Matrix4::from_col_arrays(old_matrix.model_matrix) * matrix),
                );
                instances.modified = true;
            }

            if let Some(matrix) = world_matrix {
                let instance = instances.lookup(id, mesh_id);

                let old_matrix = self.world_matrices.buffer[instance.world_matrix_index as usize];
                instance.world_matrix_index = self.world_matrices.modify_matrix(
                    instance.world_matrix_index,
                    gpu::data::GpuGameWorldMatrixData::new(Matrix4::from_col_arrays(old_matrix.world_matrix) * matrix),
                );
                instances.modified = true;
            }
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        model_man: &ModelManager,
        id: GameInstanceId<Index>,
        GameDrawInstance {
            color_offset,
            alpha,
            model_matrix,
            world_matrix,
        }: GameDrawInstance,
    ) {
        for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
            let instances = if model_man.meshes[&mesh_id].metadata.opaque {
                &mut self.opaque
            } else {
                &mut self.non_opaque
            };

            self.animations.ensure_animation_exists(model_man, mesh_id);

            instances.add(
                id,
                gpu::data::GpuGameDrawInstance {
                    mesh_id,
                    color_offset,
                    alpha,
                    model_matrix_index: self
                        .model_matrices
                        .insert_matrix(gpu::data::GpuGameModelMatrixData::new(model_matrix)),
                    world_matrix_index: self
                        .world_matrices
                        .insert_matrix(gpu::data::GpuGameWorldMatrixData::new(world_matrix)),
                    animation_index: self.animations.get(mesh_id),
                },
            );
        }
    }

    #[inline]
    pub fn remove(&mut self, id: GameInstanceId<Index>) {
        self.to_remove.insert(id);
    }

    #[inline]
    pub fn flush_removal(&mut self, model_man: &ModelManager) {
        if !self.to_remove.is_empty() {
            // reuse allocation
            {
                self.model_matrix_to_remove.clear();
                self.world_matrix_to_remove.clear();
                for &id in &self.to_remove {
                    for &mesh_id in model_man.get_model_mesh_ids(&id.model_id) {
                        let is_opaque = model_man.meshes[&mesh_id].metadata.opaque;
                        let instances = if is_opaque { &mut self.opaque } else { &mut self.non_opaque };

                        let instance = instances.lookup(id, mesh_id);
                        self.model_matrix_to_remove.insert(instance.model_matrix_index);
                        self.world_matrix_to_remove.insert(instance.world_matrix_index);

                        instances.mark_removal(id, mesh_id);
                    }
                }
                self.to_remove.clear();
            }

            // reuse allocation
            {
                self.model_matrix_remapping.clear();
                self.world_matrix_remapping.clear();

                self.model_matrices
                    .remove_matrices(self.model_matrix_to_remove.iter(), &mut self.model_matrix_remapping);
                self.world_matrices
                    .remove_matrices(self.world_matrix_to_remove.iter(), &mut self.world_matrix_remapping);

                self.opaque.flush_removal();
                self.opaque
                    .remap_indices(&self.model_matrix_remapping, &self.world_matrix_remapping);

                self.non_opaque.flush_removal();
                self.non_opaque
                    .remap_indices(&self.model_matrix_remapping, &self.world_matrix_remapping);
            }
        }
    }

    #[inline]
    fn rebuild(&mut self, model_man: &ModelManager) {
        //self.opaque.instance_offset = 0;
        self.opaque.rebuild(model_man);

        {
            if self.non_opaque.instance_offset != self.opaque.cpu_buffer_len() {
                self.non_opaque.should_rebuild_buf = true;
            }
            self.non_opaque.instance_offset = self.opaque.cpu_buffer_len();
        }
        self.non_opaque.rebuild(model_man);
    }

    #[inline]
    #[track_caller]
    pub fn mark_uploaded(&mut self) {
        self.model_matrices.modified = false;
        self.world_matrices.modified = false;

        self.opaque.modified = false;
        self.non_opaque.modified = false;
    }

    #[inline]
    #[track_caller]
    pub fn mark_dirty(&mut self) {
        self.model_matrices.modified = true;
        self.world_matrices.modified = true;

        self.opaque.mark_dirty();
        self.non_opaque.mark_dirty();
    }

    #[inline]
    pub fn to_game_draw_instances(&self, model_man: &ModelManager) -> Vec<(GameInstanceId<Index>, GameDrawInstance)> {
        let mut buffer = Vec::new();

        let mut inserted_model_ids = BTreeSet::new();
        buffer.append(&mut self.opaque.to_game_draw_instances(
            model_man,
            &self.model_matrices,
            &self.world_matrices,
            &mut inserted_model_ids,
        ));
        buffer.append(&mut self.non_opaque.to_game_draw_instances(
            model_man,
            &self.model_matrices,
            &self.world_matrices,
            &mut inserted_model_ids,
        ));

        buffer
    }

    #[inline]
    #[track_caller]
    pub fn pre_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        global_res: &gpu::GlobalResources,
        game_res: &mut gpu::GameRenderResources,
        model_man: &mut ModelManager,
        frame_start: Instant,
    ) {
        self.animations.progress_animation(model_man, frame_start);

        self.flush_removal(model_man);
        self.rebuild(model_man);

        {
            let model_matrices = bytemuck::cast_slice(&self.model_matrices.buffer);
            let world_matrices = bytemuck::cast_slice(&self.world_matrices.buffer);
            let animation_matrices = bytemuck::cast_slice(&self.animations.buffer);
            let opaque_instances = bytemuck::cast_slice(&self.opaque.buffer);
            let non_opaque_instances = bytemuck::cast_slice(&self.non_opaque.buffer);

            let model_matrices_size = model_matrices.len() as u64;
            let world_matrices_size = world_matrices.len() as u64;
            let animation_matrices_size = animation_matrices.len() as u64;
            let opaque_instances_size = self.opaque.gpu_buffer_len();
            let non_opaque_instances_size = self.non_opaque.gpu_buffer_len();
            let instances_size = opaque_instances_size + non_opaque_instances_size;

            {
                let model_matrix_buffer_size = game_res.game_pipeline.model_matrix_buffer.size();
                let world_matrix_buffer_size = game_res.game_pipeline.world_matrix_buffer.size();
                let animation_matrix_buffer_size = game_res.game_pipeline.animation_matrix_buffer.size();
                let instance_buffer_size = game_res.game_pipeline.instance_buffer.size();

                let recreate_model_matrices = model_matrices_size > model_matrix_buffer_size;
                let recreate_world_matrices = world_matrices_size > world_matrix_buffer_size;
                let recreate_animation_matrices = animation_matrices_size > animation_matrix_buffer_size;
                let recreate_instances = instances_size > instance_buffer_size;

                if recreate_model_matrices || recreate_world_matrices || recreate_animation_matrices || recreate_instances {
                    #[cfg(feature = "profile")]
                    profiling::scope!("recreate_game_pipeline");
                    game_res.game_pipeline = gpu::pipeline::GamePipeline::new(
                        device,
                        global_res,
                        gpu::pipeline::GamePipelineArgs {
                            buffer: gpu::pipeline::GamePipelineBuffersArgs {
                                model_matrix_buffer: gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                                    min_size: gpu::pipeline::new_bufer_size(model_matrix_buffer_size, model_matrices_size),
                                    buffer: game_res.game_pipeline.model_matrix_buffer.clone(),
                                    cpu_buffer: &[(0, model_matrices)],
                                },
                                world_matrix_buffer: gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                                    min_size: gpu::pipeline::new_bufer_size(world_matrix_buffer_size, world_matrices_size),
                                    buffer: game_res.game_pipeline.world_matrix_buffer.clone(),
                                    cpu_buffer: &[(0, world_matrices)],
                                },
                                animation_matrix_buffer: gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                                    min_size: gpu::pipeline::new_bufer_size(animation_matrix_buffer_size, animation_matrices_size),
                                    buffer: game_res.game_pipeline.animation_matrix_buffer.clone(),
                                    cpu_buffer: &[(0, animation_matrices)],
                                },

                                instance_buffer: gpu::pipeline::BufferInitArg::RecreateIfNeeded {
                                    min_size: instances_size,
                                    buffer: game_res.game_pipeline.instance_buffer.clone(),
                                    cpu_buffer: &[(0, opaque_instances), (opaque_instances_size, non_opaque_instances)],
                                },
                                opaque_draw_buffer: Some(game_res.game_pipeline.opaque_draw_buffer.clone()),
                                non_opaque_draw_buffer: Some(game_res.game_pipeline.non_opaque_draw_buffer.clone()),
                            },
                        },
                    );
                }

                if !recreate_model_matrices && self.model_matrices.modified {
                    #[cfg(feature = "profile")]
                    profiling::scope!("upload_model_matrices");
                    gpu::util::upload_buffer(queue, &game_res.game_pipeline.model_matrix_buffer, 0, model_matrices);
                }
                if !recreate_world_matrices && self.world_matrices.modified {
                    #[cfg(feature = "profile")]
                    profiling::scope!("upload_world_matrices");
                    gpu::util::upload_buffer(queue, &game_res.game_pipeline.world_matrix_buffer, 0, world_matrices);
                }

                if !recreate_instances {
                    if self.opaque.modified {
                        #[cfg(feature = "profile")]
                        profiling::scope!("upload_opaque_instances");
                        gpu::util::upload_buffer(queue, &game_res.game_pipeline.instance_buffer, 0, opaque_instances);
                    }
                    if self.non_opaque.modified {
                        #[cfg(feature = "profile")]
                        profiling::scope!("upload_non_opaque_instances");
                        gpu::util::upload_buffer(
                            queue,
                            &game_res.game_pipeline.instance_buffer,
                            opaque_instances_size,
                            non_opaque_instances,
                        );
                    }
                }
            }

            self.model_matrices.modified = false;
            self.world_matrices.modified = false;

            self.opaque.modified = false;
            self.non_opaque.modified = false;
        }
    }

    #[inline]
    #[track_caller]
    #[must_use]
    pub fn collect_draw_calls(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        game_res: &mut gpu::GameRenderResources,
    ) -> data::GameInstancedDrawCallsCollection<'_> {
        let draw_calls = data::GameInstancedDrawCallsCollection {
            opaque: self.opaque.get_draws(),
            non_opaque: self.non_opaque.get_draws(),
        };

        {
            if draw_calls.opaque.modified {
                #[cfg(feature = "profile")]
                profiling::scope!("upload_opaque_draws");
                gpu::util::upload_buffer_or_recreate(
                    device,
                    queue,
                    &mut game_res.game_pipeline.opaque_draw_buffer,
                    bytemuck::cast_slice(draw_calls.opaque.buffer),
                );
            }

            if draw_calls.non_opaque.modified {
                #[cfg(feature = "profile")]
                profiling::scope!("upload_non_opaque_draws");
                gpu::util::upload_buffer_or_recreate(
                    device,
                    queue,
                    &mut game_res.game_pipeline.non_opaque_draw_buffer,
                    bytemuck::cast_slice(draw_calls.non_opaque.buffer),
                );
            }
        }

        draw_calls
    }
}
