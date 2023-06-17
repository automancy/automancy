use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::IdRaw;
use automancy_defs::ply_rs::parser::Parser;
use automancy_defs::rendering::{Face, GameVertex, Model};
use automancy_defs::{id, log};
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{read_to_string, File};
use std::io::BufReader;
use std::path::Path;

use crate::{load_recursively, ResourceManager, JSON_EXT};

#[derive(Debug, Default, Clone, Copy)]
pub struct Mesh {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelJson {
    pub id: IdRaw,
    pub file: String,
}

impl ResourceManager {
    fn load_model(&mut self, file: &Path) -> Option<()> {
        log::info!("loading model at: {file:?}");

        let model: ModelJson = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let file = file
            .parent()
            .unwrap()
            .join("files")
            .join(model.file.as_str());

        log::info!("loading model file at: {file:?}");

        let file = File::open(file).unwrap();
        let mut read = BufReader::new(file);

        let vertex_parser = Parser::<GameVertex>::new();
        let face_parser = Parser::<Face>::new();

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

        load_recursively(&models, OsStr::new(JSON_EXT))
            .into_iter()
            .for_each(|file| {
                self.load_model(&file);
            });

        Some(())
    }

    pub fn compile_models(&mut self) {
        let mut ids = self
            .registry
            .tiles
            .iter()
            .flat_map(|(id, _)| self.interner.resolve(*id))
            .map(IdRaw::parse)
            .collect::<Vec<_>>();

        ids.sort_unstable();

        if let Some(none_idx) =
            ids.iter().enumerate().find_map(
                |(idx, id)| {
                    if id == &id::NONE {
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

        self.ordered_tiles = ids;

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

        let (faces, meshes): (Vec<_>, Vec<_>) = raw_faces
            .into_iter()
            .enumerate()
            .filter_map(|(i, (id, raw_faces))| {
                let face = raw_faces
                    .into_iter()
                    .map(|face| face.index_offset(index_offsets[i] as u32))
                    .reduce(|mut a, mut b| {
                        a.indices.append(&mut b.indices);

                        a
                    });

                face.map(|face| (*id, face))
            })
            .map(|(id, face)| {
                let size: u32 = face.indices.len() as u32;

                let mesh = Mesh {
                    offset: offset_count,
                    size,
                };

                offset_count += mesh.size;

                (face, (id, mesh))
            })
            .unzip();

        let meshes = HashMap::from_iter(meshes.into_iter());

        self.meshes = meshes;
        self.all_vertices = all_vertices;
        self.faces = faces;
    }
}
