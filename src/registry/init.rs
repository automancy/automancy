use std::{collections::HashMap, ops::Range};

use crate::{
    game::render::data::{Face, Vertex},
    util::resource::Resource,
};

pub struct InitData {
    pub resources: Vec<Resource>,
    pub resources_map: HashMap<&'static str, usize>,

    pub all_faces: Vec<Vec<Face>>,
    pub all_index_ranges: Vec<Vec<Range<u32>>>,
    pub combined_vertices: Vec<Vertex>,
}

impl InitData {
    pub fn new(mut resources: Vec<(&'static str, Resource)>) -> Self {
        let mut resources_map: HashMap<&'static str, usize> = HashMap::new();

        // register
        resources
            .iter_mut()
            .enumerate()
            .for_each(|(index, (id, r))| {
                r.register(index);
                resources_map.insert(id.clone(), index);
            });
        let resources = resources.into_iter().map(|(_, r)| r).collect::<Vec<_>>();

        // indices vertices
        let (vertices, faces): (Vec<_>, Vec<_>) = resources
            .iter()
            .map(|r| (r.mesh.vertices.clone(), r.mesh.faces.clone()))
            .unzip();

        let combined_vertices = vertices.into_iter().flatten().collect::<Vec<_>>();

        let mut all_faces = Vec::with_capacity(faces.len());

        faces.into_iter().fold(0, |offset, faces| {
            let offsetted_faces = faces
                .into_iter()
                .map(|face| {
                    let vertex_indices = face
                        .vertex_indices
                        .into_iter()
                        .map(|v| v + offset)
                        .collect::<Vec<_>>();

                    Face { vertex_indices }
                })
                .collect::<Vec<_>>();

            let new_offset = offsetted_faces
                .iter()
                .map(|v| v.vertex_indices.iter().max().unwrap_or(&0))
                .max()
                .unwrap_or(&offset)
                .to_owned();

            all_faces.push(offsetted_faces);

            new_offset + 1
        });

        let mut all_index_ranges = Vec::with_capacity(all_faces.len());
        all_faces.iter().fold(0, |start, faces| {
            let mut index_ranges = Vec::with_capacity(faces.len());

            let end = faces.iter().fold(start, |start, face| {
                let end = start + face.vertex_indices.len() as u32;

                index_ranges.push(start..end);

                end
            });
            all_index_ranges.push(index_ranges);

            end
        });

        log::debug!("all_index_ranges: {:?}", all_index_ranges);
        log::debug!("all_faces: {:?}", all_faces);

        InitData {
            resources,
            resources_map,
            all_faces,
            all_index_ranges,
            combined_vertices,
        }
    }
}
