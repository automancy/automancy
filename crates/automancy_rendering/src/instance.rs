use core::mem;

use automancy_data::{
    id::{ModelId, RenderId},
    rendering::gpu::GameDrawInstance,
};
use bit_set::BitSet;
use hashbrown::{HashMap, HashSet};
use slice_group_by::GroupBy;
use smallvec::SmallVec;

use crate::{AutomancyRenderer, model::GlobalMeshId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BufferLocationInfo {
    first_index: usize,
    instance_count: usize,
}

#[derive(Debug)]
pub struct DrawInstanceManager {
    render_id_map: HashMap<(RenderId, ModelId), HashSet<GlobalMeshId>>,

    instances: Vec<(GlobalMeshId, GameDrawInstance)>,
    id_locations: HashMap<GlobalMeshId, BufferLocationInfo>,
    changes: BitSet<usize>,

    staging: SmallVec<[(GlobalMeshId, GameDrawInstance); 32]>,
}

impl Default for DrawInstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawInstanceManager {
    pub fn new() -> Self {
        Self {
            render_id_map: HashMap::default(),

            instances: Vec::with_capacity(256),
            id_locations: HashMap::default(),
            changes: BitSet::default(),

            staging: SmallVec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.render_id_map.clear();
        self.instances.clear();
        self.id_locations.clear();
        self.changes.clear();
        self.staging.clear();
    }
}

impl DrawInstanceManager {
    #[cfg(debug_assertions)]
    pub fn check_validity(&self) {
        let mut correct_locs = HashMap::new();
        let mut index = 0;
        for group in self.instances.binary_group_by_key(|v| v.0) {
            let id = group[0].0;

            if correct_locs.contains_key(&id) {
                panic!("instances aren't in sequence");
            }

            let vec = group.iter().map(|v| v.1).collect::<Vec<_>>();

            correct_locs.insert(
                id,
                BufferLocationInfo {
                    first_index: index,
                    instance_count: vec.len(),
                },
            );

            index += vec.len();
        }

        for (id, correct_loc) in &correct_locs {
            assert!(self.id_locations.contains_key(id));
            assert_eq!(*correct_loc, self.id_locations[id]);
        }

        for (id, loc) in &self.id_locations {
            assert!(correct_locs.contains_key(id));
            assert_eq!(correct_locs[id], *loc);
        }
    }

    #[cfg(debug_assertions)]
    pub fn checked_insert(
        &mut self,
        renderer: &AutomancyRenderer,
        tag: RenderId,
        model_id: ModelId,
        instance: GameDrawInstance,
    ) {
        let staging_len = self.staging.len();
        let changes = self.changes.clone();
        let instances = self.instances.clone();

        self.insert(renderer, tag, model_id, instance);
        if self.staging.len() < staging_len {
            for change in self.changes.difference(&changes) {
                assert_ne!(self.instances.get(change), instances.get(change));
            }
        } else {
            assert_eq!(self.instances, instances);
        }
        self.check_validity();
    }

    fn splice_in(
        &mut self,
        start: usize,
        id: GlobalMeshId,
        instances: impl ExactSizeIterator<Item = GameDrawInstance>,
    ) {
        let instance_count = instances.len();

        self.changes
            .extend(start..(self.instances.len() + instance_count));

        for id in self.instances[start..]
            .iter()
            .map(|(id, _)| *id)
            .collect::<HashSet<_>>()
        {
            self.id_locations.get_mut(&id).unwrap().first_index += instance_count;
        }

        self.id_locations.get_mut(&id).unwrap().instance_count += instance_count;

        self.instances
            .splice(start..start, instances.map(|v| (id, v)));
    }

    fn push(
        &mut self,
        id: GlobalMeshId,
        instances: impl ExactSizeIterator<Item = GameDrawInstance>,
    ) {
        let end_of_buffer = self.instances.len();
        let instance_count = instances.len();

        self.changes
            .extend(end_of_buffer..(end_of_buffer + instance_count));
        if !self.id_locations.contains_key(&id) {
            self.id_locations.insert(
                id,
                BufferLocationInfo {
                    first_index: end_of_buffer,
                    instance_count,
                },
            );
        } else {
            debug_assert!(self.id_locations.get_mut(&id).unwrap().first_index < end_of_buffer);

            self.id_locations.get_mut(&id).unwrap().instance_count += instance_count;
        }
        self.instances.extend(instances.map(|v| (id, v)));
    }

    fn swap_in(&mut self, mut instances: Vec<(GlobalMeshId, GameDrawInstance)>) {
        let instances = {
            instances.sort_by_key(|(id, ..)| *id);

            let mut buffers = Vec::new();
            {
                let mut curr_id = None;
                let mut temp_buffer = Vec::new();
                for instance @ (id, ..) in instances {
                    if curr_id.is_none() {
                        curr_id = Some(id);
                    }

                    if id != curr_id.unwrap() {
                        buffers.push(mem::take(&mut temp_buffer));
                        curr_id = Some(id);
                    }

                    temp_buffer.push(instance);
                }
                buffers.push(temp_buffer);
            }

            buffers.sort_by_key(|v| v.len());
            buffers.reverse();

            buffers.into_iter().flatten().collect::<Vec<_>>()
        };

        let mut swapped_out = Vec::new();
        for batch in instances.binary_group_by_key(|(id, ..)| *id) {
            let id = batch[0].0;
            let instances = batch.iter().map(|(_id, instance)| *instance);

            match self.id_locations.get(&id).cloned() {
                None => {
                    self.push(id, instances);
                }
                Some(loc) => {
                    debug_assert_ne!(self.instances.len(), 0);

                    let instance_count = instances.len();
                    debug_assert_ne!(0, instance_count);

                    // the starting location for inserting the instances in (aka right after the last instance of the same mesh id)
                    let start = loc.first_index + loc.instance_count;

                    // the remaining items after `start`
                    let remaining_items = self.instances.len() - start;

                    // if we're *at* the end of the buffer, then just push.
                    if remaining_items == 0 {
                        self.push(id, instances);
                        continue;
                    }

                    // if we're *near* the end of the buffer, then just splice it in.
                    if remaining_items <= instance_count {
                        self.splice_in(start, id, instances);
                        continue;
                    }

                    // swap in the new instances
                    self.changes.extend(start..(start + instance_count));
                    self.id_locations.get_mut(&id).unwrap().instance_count += instance_count;
                    let swapped_instances = self
                        .instances
                        .splice(start..(start + instance_count), instances.map(|v| (id, v)))
                        .collect::<Vec<_>>();
                    debug_assert_eq!(swapped_instances.len(), instance_count);

                    let mut map = HashMap::<GlobalMeshId, Vec<GameDrawInstance>>::new();
                    for (id, instance) in swapped_instances {
                        map.entry(id).or_default().push(instance);
                    }

                    for (id, swapped_instances) in map {
                        // only do this if we've removed every of some instances by swapping
                        if swapped_instances.len() == self.id_locations[&id].instance_count {
                            self.id_locations.remove(&id);
                            self.push(id, swapped_instances.into_iter());
                        } else {
                            // we necessarily swapped out the instances from the back
                            self.id_locations.get_mut(&id).unwrap().first_index +=
                                swapped_instances.len();
                            self.id_locations.get_mut(&id).unwrap().instance_count -=
                                swapped_instances.len();
                            swapped_out.extend(swapped_instances.into_iter().map(|v| (id, v)));
                        }
                    }
                }
            }
        }
    }

    pub fn insert(
        &mut self,
        renderer: &AutomancyRenderer,
        tag: RenderId,
        model_id: ModelId,
        instance: GameDrawInstance,
    ) {
        if self.staging.len() < self.staging.capacity() {
            for &mesh_id in renderer.model_meshes.get(&model_id).unwrap().iter() {
                let global_id = renderer.global_id_map[&(model_id, mesh_id)];

                self.render_id_map
                    .entry((tag, model_id))
                    .or_default()
                    .insert(global_id);

                self.staging.push((global_id, instance))
            }
        } else {
            let instances = mem::take(&mut self.staging);

            self.swap_in(instances.into_vec());
        }
    }

    pub fn take_changes(&mut self) -> BitSet<usize> {
        mem::take(&mut self.changes)
    }
}
