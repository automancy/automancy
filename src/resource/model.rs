use crate::render::data::{Model, RawFace, Vertex};
use crate::resource::ResourceManager;
use crate::resource::JSON_EXT;
use crate::util::id::IdRaw;
use ply_rs::parser::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string, File};
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Face {
    pub offset: u32,
    pub size: u32,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ModelRaw {
    pub id: IdRaw,
    pub file: String,
}

impl ResourceManager {
    fn load_model(&mut self, file: &Path) -> Option<()> {
        log::info!("loading model at: {file:?}");

        let model: ModelRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let file = file
            .parent()
            .unwrap()
            .join("files")
            .join(model.file.as_str());

        log::info!("loading model file at: {file:?}");

        let file = File::open(file).ok().unwrap();
        let mut read = BufReader::new(file);

        let vertex_parser = Parser::<Vertex>::new();
        let face_parser = Parser::<RawFace>::new();

        let header = vertex_parser.read_header(&mut read).unwrap();

        let mut vertices = None;
        let mut faces = None;

        for (_, element) in &header.elements {
            match element.name.as_ref() {
                "vertex" => {
                    vertices = vertex_parser
                        .read_payload_for_element(&mut read, element, &header)
                        .ok();
                }
                "face" => {
                    faces = face_parser
                        .read_payload_for_element(&mut read, element, &header)
                        .ok();
                }
                _ => (),
            }
        }

        let raw_model = vertices
            .zip(faces)
            .map(|(vertices, faces)| Model::new(vertices, faces))?;

        self.raw_models
            .insert(model.id.to_id(&mut self.interner), raw_model);

        Some(())
    }
    pub fn load_models(&mut self, dir: &Path) -> Option<()> {
        let models = dir.join("models");
        let models = read_dir(models).ok()?;

        models
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|model| {
                self.load_model(&model);
            });

        Some(())
    }
    pub fn compile_models(&mut self) {
        let mut ids = self
            .tiles
            .iter()
            .flat_map(|(id, _)| self.interner.resolve(*id))
            .map(IdRaw::parse)
            .collect::<Vec<_>>();

        ids.sort_unstable_by_key(|id| id.clone());

        if let Some(none_idx) =
            ids.iter().enumerate().find_map(
                |(idx, id)| {
                    if id == &IdRaw::NONE {
                        Some(idx)
                    } else {
                        None
                    }
                },
            )
        {
            ids.swap(none_idx, 0);
        }

        let ids = ids
            .into_iter()
            .flat_map(|id| self.interner.get(id.to_string()))
            .collect();

        self.ordered_ids = ids;

        // indices vertices
        let (vertices, raw_faces): (Vec<_>, Vec<_>) = self
            .raw_models
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

        let all_vertices = vertices.into_iter().flatten().collect::<Vec<_>>();

        let mut offset_count = 0;

        let (raw_faces, faces): (Vec<_>, Vec<_>) = raw_faces // TODO we can just draw 3 indices a bunch of times
            .into_iter()
            .enumerate()
            .map(|(i, (id, raw_faces))| {
                let raw_faces = raw_faces
                    .into_iter()
                    .map(|face| face.index_offset(index_offsets[i] as u32))
                    .collect::<Vec<_>>();

                let faces = raw_faces
                    .iter()
                    .map(|face| {
                        let size = face.indices.len() as u32;
                        let offset = offset_count;

                        offset_count += size;

                        Face { size, offset }
                    })
                    .collect::<Vec<_>>();

                (raw_faces, (*id, faces))
            })
            .unzip();

        let faces = HashMap::from_iter(faces.into_iter());

        /*
        log::debug!("combined_vertices: {:?}", combined_vertices);
        log::debug!("all_raw_faces: {:?}", all_raw_faces);
        log::debug!("all_faces: {:?}", all_faces);
         */

        self.faces = faces;
        self.all_vertices = all_vertices;
        self.raw_faces = raw_faces;
    }
}
