use crate::{
    render::data::{Face, Model, Vertex},
};
use crate::data::id::Id;
use crate::util::resource::ResourceManager;

#[derive(Debug)]
pub struct InitData {
    pub resource_man: ResourceManager,

    pub all_faces: Vec<Option<Vec<Face>>>,
    pub combined_vertices: Vec<Vertex>,
}

impl InitData {
    pub fn new(
        mut resource_man: ResourceManager,
        resources: Vec<(Id, Option<Model>)>,
    ) -> Self {
        // indices vertices
        let (vertices, faces): (Vec<_>, Vec<_>) = resources
            .into_iter()
            .map(|(id, model)| model.map(|m| (m.vertices, (id, m.faces))))
            .map(|v| v.unzip())
            .unzip();

        let mut index_offsets = vertices
            .iter()
            .scan(0, |offset, v| {
                if let Some(ref v) = v {
                    *offset += v.len();
                    Some(*offset)
                } else {
                    Some(0)
                }
            })
            .collect::<Vec<_>>();
        if !index_offsets.is_empty() {
            drop(index_offsets.split_off(index_offsets.len() - 1));
            index_offsets.insert(0, 0);
        }

        let combined_vertices = vertices.into_iter().flatten().flatten().collect::<Vec<_>>();

        let mut offset = 0;

        let all_faces: Vec<_> = faces
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.map(|(id, faces)| {
                    if let Some(resource) = resource_man.resources.get_mut(&id) {
                        resource.faces_index = Some(i);
                    }

                    faces
                        .into_iter()
                        .map(|face| {
                            let len = face.indices.len();

                            let mut result = face.with_offset(offset);

                                result.index_offset(index_offsets[i] as u32);
                            offset += len as u32;

                            result
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        InitData {
            resource_man,

            all_faces,
            combined_vertices,
        }
    }
}
