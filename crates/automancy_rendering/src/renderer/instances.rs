use std::{collections::BTreeMap, time::Instant};

use automancy_data::{
    id::{ModelId, RenderId},
    math::Matrix4,
    rendering::gpu::GameDrawInstance,
};
use hashbrown::{HashMap, HashSet};
use slice_group_by::GroupBy;

use crate::{
    data::{GpuAnimationMatrixData, GpuDrawInstance, GpuModelMatrixData, GpuWorldMatrixData},
    gpu,
    model::{GlobalMeshId, ModelManager},
};

#[derive(Debug)]
pub struct DrawInstanceManager {
    to_remove: HashSet<(RenderId, ModelId)>,

    opaque_instance_index_map: HashMap<(RenderId, GlobalMeshId), Vec<u32>>,
    opaque_instance_buffer: Vec<((RenderId, GlobalMeshId), GpuDrawInstance)>,
    non_opaque_instance_index_map: HashMap<(RenderId, GlobalMeshId), Vec<u32>>,
    non_opaque_instance_buffer: Vec<((RenderId, GlobalMeshId), GpuDrawInstance)>,

    model_matrices: Vec<GpuModelMatrixData>,
    model_matrices_reuses: HashMap<u32, u32>,
    world_matrices: Vec<GpuWorldMatrixData>,
    world_matrices_reuses: HashMap<u16, u16>,

    animation_ids: BTreeMap<GlobalMeshId, u16>,
    animation_matrices: Vec<GpuAnimationMatrixData>,
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

            opaque_instance_index_map: Default::default(),
            opaque_instance_buffer: Vec::with_capacity(256),
            non_opaque_instance_index_map: Default::default(),
            non_opaque_instance_buffer: Vec::with_capacity(64),

            model_matrices: Vec::with_capacity(256),
            model_matrices_reuses: Default::default(),
            world_matrices: Vec::new(),
            world_matrices_reuses: Default::default(),

            animation_ids: Default::default(),
            animation_matrices: Vec::with_capacity(128),
        }
    }

    pub fn clear(&mut self) {
        self.animation_ids.clear();
        self.to_remove.clear();

        self.opaque_instance_buffer.clear();
        self.model_matrices.clear();
        self.world_matrices.clear();
        self.animation_matrices.clear();
    }
}

impl DrawInstanceManager {
    #[inline]
    pub fn inspect_instances(
        &mut self,
        model_man: &ModelManager,
        render_id: RenderId,
        model_id: ModelId,
        mut f: impl FnMut(GlobalMeshId, &mut GpuDrawInstance),
    ) {
        for global_id in &model_man.global_model_mesh_ids[&model_id] {
            let id = (render_id, *global_id);

            if let Some(indices) = self
                .opaque_instance_index_map
                .get(&id)
                .or(self.non_opaque_instance_index_map.get(&id))
            {
                for i in indices {
                    let ((_, mesh), instance) =
                        self.opaque_instance_buffer.get_mut(*i as usize).unwrap();

                    f(*mesh, instance)
                }
            }
        }
    }

    #[inline]
    pub fn set_all_world_matrix(&mut self, matrix: Matrix4) {
        let matrix = GpuWorldMatrixData::new(matrix);
        for v in self.world_matrices.iter_mut() {
            *v = matrix;
        }
    }

    #[inline]
    pub fn set_matrix(
        &mut self,
        model_man: &ModelManager,
        render_id: RenderId,
        model_id: ModelId,
        model_matrix: Option<Matrix4>,
        world_matrix: Option<Matrix4>,
    ) {
        for global_id in &model_man.global_model_mesh_ids[&model_id] {
            let id = (render_id, *global_id);

            if let Some(indices) = self
                .opaque_instance_index_map
                .get(&id)
                .or(self.non_opaque_instance_index_map.get(&id))
            {
                for index in indices {
                    let (_, instance) = self
                        .opaque_instance_buffer
                        .get_mut(*index as usize)
                        .unwrap();

                    if let Some(matrix) = model_matrix {
                        instance.model_matrix_index = util::modify_matrix(
                            &mut self.model_matrices,
                            &mut self.model_matrices_reuses,
                            GpuModelMatrixData::new(matrix),
                            instance.model_matrix_index,
                        );
                    }

                    if let Some(matrix) = world_matrix {
                        instance.world_matrix_index = util::modify_matrix(
                            &mut self.world_matrices,
                            &mut self.world_matrices_reuses,
                            GpuWorldMatrixData::new(matrix),
                            instance.world_matrix_index,
                        );
                    }
                }
            }
        }
    }
}

impl DrawInstanceManager {
    #[inline]
    fn ensure_animation_exists(&mut self, global_id: GlobalMeshId) {
        if !self.animation_ids.contains_key(&global_id) {
            let i = self.animation_matrices.len() as u16;
            self.animation_matrices
                .push(GpuAnimationMatrixData::default());

            self.animation_ids.insert(global_id, i);
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        model_man: &ModelManager,
        render_id: RenderId,
        model_id: ModelId,
        GameDrawInstance {
            color_offset,
            alpha,
            model_matrix,
            world_matrix,
        }: GameDrawInstance,
    ) {
        let model_matrix_index = util::insert_matrix(
            &mut self.model_matrices,
            &mut self.model_matrices_reuses,
            GpuModelMatrixData::new(model_matrix),
        );

        let world_matrix_index = util::insert_matrix(
            &mut self.world_matrices,
            &mut self.world_matrices_reuses,
            GpuWorldMatrixData::new(world_matrix),
        );

        for &global_id in &model_man.global_model_mesh_ids[&model_id] {
            self.ensure_animation_exists(global_id);

            let is_opaque = model_man.mesh_metadata.get(&global_id).unwrap().opaque;

            let buffer = if is_opaque {
                &mut self.opaque_instance_buffer
            } else {
                &mut self.non_opaque_instance_buffer
            };

            let index_map = if is_opaque {
                &mut self.opaque_instance_index_map
            } else {
                &mut self.non_opaque_instance_index_map
            };

            let new_index = buffer.len() as u32;
            buffer.push((
                (render_id, global_id),
                GpuDrawInstance::new(
                    color_offset,
                    alpha,
                    model_matrix_index,
                    world_matrix_index,
                    self.animation_ids[&global_id],
                ),
            ));
            index_map
                .entry((render_id, global_id))
                .or_default()
                .push(new_index);
        }
    }

    #[inline]
    pub fn remove(&mut self, render_id: RenderId, model_id: ModelId) {
        self.to_remove.insert((render_id, model_id));
    }

    #[inline]
    pub fn upload_animation(
        &mut self,
        start_instant: Instant,
        model_man: &ModelManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        animation_matrix_data_buffer: &mut wgpu::Buffer,
    ) {
        let elapsed = Instant::now().duration_since(start_instant).as_secs_f32();

        for (&global_id, &index) in &self.animation_ids {
            if let Some(animation) = model_man.animation_channels_map.get(&global_id) {
                let wrapped = elapsed % animation.duration;
                debug_assert!(!animation.keyframes.is_empty());

                let anim_pivot = animation.keyframes.partition_point(|(v, ..)| *v < wrapped);
                let matrix = animation.keyframes[anim_pivot - 1].1;

                self.animation_matrices[index as usize] = GpuAnimationMatrixData::new(matrix);
            }
        }

        gpu::util::upload_buffer(
            device,
            queue,
            animation_matrix_data_buffer,
            &self.animation_matrices,
        );
    }

    #[inline]
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn collect_draw_calls(
        &mut self,
        model_man: &ModelManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model_matrix_data_buffer: &mut wgpu::Buffer,
        world_matrix_data_buffer: &mut wgpu::Buffer,
        opaque_instance_buffer: &mut wgpu::Buffer,
        non_opaque_instance_buffer: &mut wgpu::Buffer,
    ) -> [Vec<wgpu::util::DrawIndexedIndirectArgs>; 2] {
        let mut to_remove = HashSet::new();
        for (render_id, model_id) in std::mem::take(&mut self.to_remove) {
            for &global_id in &model_man.global_model_mesh_ids[&model_id] {
                to_remove.insert((render_id, global_id));
            }
        }

        fn remove(
            to_remove: &HashSet<(RenderId, GlobalMeshId)>,
            instance_buffer: Vec<((RenderId, GlobalMeshId), GpuDrawInstance)>,
            index_map: &mut HashMap<(RenderId, GlobalMeshId), Vec<u32>>,
            model_matrices: &mut Vec<GpuModelMatrixData>,
            world_matrices: &mut Vec<GpuWorldMatrixData>,
        ) -> Vec<((RenderId, GlobalMeshId), GpuDrawInstance)> {
            let mut model_matrix_mapping = HashMap::new();
            let mut world_matrix_mapping = HashMap::new();

            for (id, instance) in &instance_buffer {
                if to_remove.contains(id) {
                    index_map.remove(id);

                    {
                        let index = instance.model_matrix_index;
                        let last_index = (model_matrices.len() - 1) as u32;

                        model_matrices.swap_remove(index as usize);

                        if index != last_index {
                            assert_eq!(None, model_matrix_mapping.insert(last_index, index));
                        }
                    }

                    {
                        let index = instance.world_matrix_index;
                        let last_index = (world_matrices.len() - 1) as u16;

                        world_matrices.swap_remove(index as usize);

                        if index != last_index {
                            assert_eq!(None, world_matrix_mapping.insert(last_index, index));
                        }
                    }
                }
            }

            instance_buffer
                .into_iter()
                .filter(|(id, _)| !to_remove.contains(id))
                .map(|(id, mut instance)| {
                    if let Some(index) = model_matrix_mapping.get(&instance.model_matrix_index) {
                        instance.model_matrix_index = *index;
                    }

                    if let Some(index) = world_matrix_mapping.get(&instance.world_matrix_index) {
                        instance.world_matrix_index = *index;
                    }

                    (id, instance)
                })
                .collect::<Vec<_>>()
        }

        self.opaque_instance_buffer = remove(
            &to_remove,
            std::mem::take(&mut self.opaque_instance_buffer),
            &mut self.opaque_instance_index_map,
            &mut self.model_matrices,
            &mut self.world_matrices,
        );
        self.non_opaque_instance_buffer = remove(
            &to_remove,
            std::mem::take(&mut self.non_opaque_instance_buffer),
            &mut self.non_opaque_instance_index_map,
            &mut self.model_matrices,
            &mut self.world_matrices,
        );

        self.opaque_instance_buffer.sort_by_key(|v| v.0.1);
        self.non_opaque_instance_buffer.sort_by_key(|v| v.0.1);

        gpu::util::upload_buffer(
            device,
            queue,
            model_matrix_data_buffer,
            &self.model_matrices,
        );
        gpu::util::upload_buffer(
            device,
            queue,
            world_matrix_data_buffer,
            &self.world_matrices,
        );

        gpu::util::upload_buffer(
            device,
            queue,
            opaque_instance_buffer,
            &self
                .opaque_instance_buffer
                .iter()
                .map(|v| v.1)
                .collect::<Vec<_>>(),
        );
        gpu::util::upload_buffer(
            device,
            queue,
            non_opaque_instance_buffer,
            &self
                .non_opaque_instance_buffer
                .iter()
                .map(|v| v.1)
                .collect::<Vec<_>>(),
        );

        fn collect(
            model_man: &ModelManager,
            instance_buffer: &[((RenderId, GlobalMeshId), GpuDrawInstance)],
        ) -> Vec<wgpu::util::DrawIndexedIndirectArgs> {
            let mut draws = Vec::with_capacity(256);
            let mut first_instance = 0;
            for instance_group in instance_buffer.binary_group_by_key(|v| v.0.1) {
                let global_id: GlobalMeshId = instance_group[0].0.1;
                let instance_count = instance_group.len() as u32;

                let index_range = model_man.index_ranges[&global_id];

                draws.push(wgpu::util::DrawIndexedIndirectArgs {
                    index_count: index_range.count,
                    first_index: index_range.pos,
                    base_vertex: index_range.base_vertex,
                    instance_count,
                    first_instance,
                });

                first_instance += instance_count
            }

            draws
        }

        [
            collect(model_man, &self.opaque_instance_buffer),
            collect(model_man, &self.non_opaque_instance_buffer),
        ]
    }
}

mod util {
    use std::hash::Hash;

    use hashbrown::HashMap;
    use num_traits::{AsPrimitive, NumAssignOps, NumOps, PrimInt};

    #[inline]
    pub(crate) fn modify_matrix<
        V: PartialEq,
        Num: PrimInt + NumOps + NumAssignOps + AsPrimitive<usize> + Hash + 'static,
    >(
        collection: &mut Vec<V>,
        reuse_counter: &mut HashMap<Num, Num>,
        matrix: V,
        index: Num,
    ) -> Num
    where
        usize: AsPrimitive<Num>,
    {
        if let Some(count) = reuse_counter.get_mut(&index) {
            // the index is reused elsewhere
            *count -= Num::one();

            let new_index = insert_matrix(collection, reuse_counter, matrix);

            if reuse_counter[&index] == Num::zero() {
                reuse_counter.remove(&index);
            }

            new_index
        } else {
            // the index is *not* reused elsewhere, so let's just modify it
            *collection.get_mut(index.as_()).unwrap() = matrix;

            index
        }
    }

    #[inline]
    pub(crate) fn insert_matrix<
        V: PartialEq,
        Num: PrimInt + NumOps + NumAssignOps + Hash + 'static,
    >(
        collection: &mut Vec<V>,
        reuse_counter: &mut HashMap<Num, Num>,
        matrix: V,
    ) -> Num
    where
        usize: AsPrimitive<Num>,
    {
        if Some(&matrix) == collection.last() {
            let i = (collection.len() - 1).as_();
            *reuse_counter.entry(i).or_insert(Num::zero()) += Num::one();

            i
        } else {
            let i = collection.len().as_();
            collection.push(matrix);

            i
        }
    }
}
