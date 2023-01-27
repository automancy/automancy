use std::{collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::Path};

use json::object::Object;
use ply_rs::parser::Parser;

use crate::{
    render::data::{Face, Model, Vertex},
    util::id::{id},
};

use super::id::{Id};

#[derive(Debug, Default, Clone)]
pub struct Resource {
    pub faces_index: Option<usize>,
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    pub resources: HashMap<Id, Resource>,
}

impl ResourceManager {
    fn parse_model(&self, json: Object, working_dir: &Path) -> Option<Model> {
        json.get("model")?
            .as_str()
            .map(|v| {
                let p = working_dir.join(v);
                log::debug!("trying to load model: {:?}", p);
                p
            })
            .map(File::open)
            .and_then(Result::ok)
            .and_then(|file| {
                let mut model_reader = BufReader::new(file);

                let vertex_parser = Parser::<Vertex>::new();
                let face_parser = Parser::<Face>::new();

                let header = vertex_parser.read_header(&mut model_reader).unwrap();

                let mut vertices = None;
                let mut faces = None;

                for (_, element) in &header.elements {
                    match element.name.as_ref() {
                        "vertex" => {
                            vertices = vertex_parser
                                .read_payload_for_element(&mut model_reader, &element, &header)
                                .ok();
                        }
                        "face" => {
                            faces = face_parser
                                .read_payload_for_element(&mut model_reader, &element, &header)
                                .ok();
                        }
                        _ => (),
                    }
                }

                vertices
                    .zip(faces)
                    .map(|(vertices, faces)| Model::new(vertices, faces))
            })
    }

    // TODO naming!!!!!

    pub fn load_resource(
        &mut self,
        json: Object,
        working_dir: &Path,
    ) -> Option<(Id, Option<Model>)> {
        let namespace = working_dir
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("automancy");
        let name = json.get("id")?.as_str()?;

        let id = id(namespace, name);

        self.resources
            .insert(id.clone(), Resource { faces_index: None });

        let model = self.parse_model(json, working_dir);

        model.as_ref().inspect(|v| {
            log::debug!("loaded model vertices: {}", v.vertices.len());
            log::debug!("loaded model faces: {}", v.faces.len());
        });

        Some((id, model))
    }
}
