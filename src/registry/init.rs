use std::{collections::HashMap, ops::Range};

use crate::{
    game::{
        data::id::Id,
        render::data::{Face, Vertex},
    },
    util::resource::Resource,
};

pub struct InitData {
    pub resources: Vec<Resource>,
    pub resources_map: HashMap<Id, usize>,

    pub all_faces: Vec<Option<Vec<Face>>>,
    pub all_index_ranges: Vec<Option<Vec<Range<u32>>>>,
    pub combined_vertices: Vec<Vertex>,
}

impl InitData {
    pub fn new(mut resources: Vec<Resource>) -> Self {
        let mut resources_map: HashMap<Id, usize> = HashMap::new();

        // register
        resources.iter_mut().enumerate().for_each(|(index, r)| {
            r.register(index);
            resources_map.insert(r.id.clone(), index);
        });

        // indices vertices
        let (vertices, faces): (Vec<_>, Vec<_>) = resources
            .iter()
            .map(|r| r.model.as_ref().and_then(|m| Some(m)))
            .map(|model| {
                (
                    model.and_then(|m| Some(m.vertices.clone())),
                    model.and_then(|m| Some(m.faces.clone())),
                )
            })
            .unzip();

        let combined_vertices = vertices.into_iter().flatten().flatten().collect::<Vec<_>>();

        let mut all_faces = Vec::with_capacity(faces.len());

        faces.into_iter().fold(0, |offset, faces| {
            let new_offset;

            all_faces.push(if let Some(faces) = faces {
                let offsetted_faces = faces
                    .into_iter()
                    .map(|face| {
                        let vertex_indices = face
                            .vertex_indices
                            .into_iter()
                            .map(|v| v + offset)
                            .collect();

                        Face { vertex_indices }
                    })
                    .collect::<Vec<_>>();

                new_offset = *offsetted_faces
                    .iter()
                    .map(|v| v.vertex_indices.iter().max().unwrap_or(&0))
                    .max()
                    .unwrap_or(&offset);

                Some(offsetted_faces)
            } else {
                new_offset = offset;

                None
            });

            new_offset + 1
        });

        let mut all_index_ranges = Vec::with_capacity(all_faces.len());
        all_faces.iter().fold(0, |start, faces| {
            if let Some(faces) = faces {
                let mut index_ranges = Vec::with_capacity(faces.len());

                let end = faces.iter().fold(start, |start, face| {
                    let end = start + face.vertex_indices.len() as u32;

                    index_ranges.push(start..end);

                    end
                });
                all_index_ranges.push(Some(index_ranges));

                return end;
            } else {
                all_index_ranges.push(None);

                return start;
            }
        });

        log::debug!("combined_vertices: {:?}", combined_vertices);
        log::debug!("all_index_ranges: {:?}", all_index_ranges);
        log::debug!("all_faces: {:?}", all_faces);
        log::debug!("all registered resources: {:?}", resources_map.keys());

        InitData {
            resources,
            resources_map,
            all_faces,
            all_index_ranges,
            combined_vertices,
        }
    }
}
