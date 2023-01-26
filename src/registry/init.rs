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

        let offsets = {
            let len = vertices.len();

            let mut vec = Vec::with_capacity(len);

            vertices
                .iter()
                .map(|v| v.as_ref().map_or(0, Vec::len))
                .for_each(|v| vec.push(v + vec.last().map_or(0, Clone::clone)));

            vec.truncate(len - 1);
            vec.insert(0, 0);

            vec
        };

        let combined_vertices = vertices.into_iter().flatten().flatten().collect::<Vec<_>>();

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
                        .map(|face| face.with_offset(offsets[i] as u32))
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        log::debug!(
            "all registered resources: {:?}",
            resource_man.resources.keys()
        );

        log::debug!("combined_vertices: {:?}", combined_vertices);
        log::debug!("all_faces: {:?}", all_faces);

        InitData {
            resource_man,

            all_faces,
            combined_vertices,
        }
    }
}
