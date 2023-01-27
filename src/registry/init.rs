use crate::{
    render::data::{Face, Model, Vertex},
    util::{
        id::{Id},
        resource::ResourceManager,
    },
};

#[derive(Debug)]
pub struct InitData {
    pub resource_man: ResourceManager,

    pub all_faces: Vec<Option<Vec<Face>>>,
    pub combined_vertices: Vec<Vertex>,
}

impl InitData {
    pub fn new(
        mut resource_man: ResourceManager,
        resources: Vec<Option<(Id, Option<Model>)>>,
    ) -> Self {
        // indices vertices
        let (vertices, faces): (Vec<_>, Vec<_>) = resources
            .into_iter()
            .map(|r| r.and_then(|(id, model)| model.map(|m| (m.vertices, (id, m.faces)))))
            .map(|v| v.unzip())
            .unzip();

        let mut index_offsets = vertices
            .iter()
            .scan(0, |offset, v| {
                v.as_ref().map(|v| {
                    *offset += v.len();
                    Some(*offset)
                })
            })
            .collect::<Vec<_>>();
        drop(index_offsets.split_off(index_offsets.len() - 1));
        index_offsets.insert(0, Some(0));

        let combined_vertices = vertices.into_iter().flatten().flatten().collect::<Vec<_>>();

        let mut offset = 0;

        let all_faces: Vec<_> = faces
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.map(|(id, faces)| {
                    if let Some(resource) = resource_man.resources.get_mut(&id) {
                        log::debug!("{}->faces_index: {}", id, i);
                        resource.faces_index = Some(i);
                    }

                    faces
                        .into_iter()
                        .map(|face| {
                            let len = face.indices.len();

                            let mut result = face.with_offset(offset);

                            if let Some(index_offset) = index_offsets[i] {
                                result.index_offset(index_offset as u32);
                            }
                            offset += len as u32;

                            result
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        log::debug!(
            "all registered resources: {:?}",
            resource_man.resources
        );

        log::debug!("combined_vertices ({}): {:?}", combined_vertices.len(), combined_vertices);
        log::debug!("all_faces:");
        resource_man.resources.iter().for_each(|(id, resource)| {
            resource.faces_index.map(|faces_index| {
                log::debug!("{} ({}):\n{:?}\n", id, faces_index, all_faces[faces_index]);
            });
        });

        InitData {
            resource_man,

            all_faces,
            combined_vertices,
        }
    }
}
