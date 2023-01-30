use crate::{
    render::data::{RawFace, Vertex},
};
use crate::data::id::Id;
use crate::util::resource::{Face, ResourceManager};

#[derive(Debug)]
pub struct InitData {
    pub resource_man: ResourceManager,

    pub all_raw_faces: Vec<Vec<RawFace>>,
    pub combined_vertices: Vec<Vertex>,
}

impl InitData {
    pub fn new(mut resource_man: ResourceManager) -> Self {
        let mut ids = resource_man.resources.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();

        ids.sort_unstable_by_key(|id| id.clone());

        if let Some(none_idx) = ids.iter().enumerate().find_map(|(idx, id)| {
            if id == &Id::NONE {
                Some(idx)
            } else {
                None
            }
        }) {
            ids.swap(none_idx, 0);
        }

        resource_man.ordered_ids = ids;

        // indices vertices
        let (vertices, raw_faces): (Vec<_>, Vec<_>) = resource_man.raw_models
            .iter()
            .map(|(id, model)| (model.vertices.clone(), (id, model.faces.clone())))
            .unzip();

        let mut index_offsets = vertices
            .iter()
            .scan(0, |offset, v| {
                *offset += v.len();
                Some(*offset)
            })
            .collect::<Vec<_>>();

        drop(index_offsets.split_off(index_offsets.len() - 1));
        index_offsets.insert(0, 0);

        let combined_vertices = vertices.into_iter().flatten().collect::<Vec<_>>();

        let mut offset_count = 0;

        let (all_raw_faces, all_faces): (Vec<_>, Vec<_>) = raw_faces // TODO we can just draw 3 indices a bunch of times
            .into_iter()
            .enumerate()
            .map(|(i, (id, raw_faces))| {
                resource_man.models_referenced
                    .get(&id)
                    .map(|references| {
                        references
                            .iter()
                            .for_each(|id| {
                                if let Some(resource) = resource_man.resources.get_mut(id) {
                                    resource.faces_index = Some(i);
                                }
                            });
                    });

                let faces = raw_faces
                    .iter()
                    .map(|face| {
                        let size = face.indices.len() as u32;
                        let offset = offset_count;

                        offset_count += size;

                        Face { size, offset }
                    })
                    .collect::<Vec<_>>();

                let raw_faces = raw_faces
                    .into_iter()
                    .map(|face| face.index_offset(index_offsets[i] as u32))
                    .collect::<Vec<_>>();

                (raw_faces, faces)
            })
            .unzip();

        log::debug!("combined_vertices: {:?}", combined_vertices);
        log::debug!("all_raw_faces: {:?}", all_raw_faces);
        log::debug!("all_faces: {:?}", all_faces);

        resource_man.all_faces = all_faces;

        InitData {
            resource_man,

            all_raw_faces,
            combined_vertices,
        }
    }
}
