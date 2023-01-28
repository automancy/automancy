use std::{collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::Path};
use std::fs::read_to_string;
use std::sync::Arc;
use json::JsonValue;

use json::object::Object;
use ply_rs::parser::Parser;
use serde::Deserialize;
use serde_json::Error;

use crate::{
    render::data::{Face, Model, Vertex},
    data::id::{Id, id},
};
use crate::game::script::Script;

#[derive(Debug, Default, Clone)]
pub struct ResourceRef {
    pub scripts: Option<Vec<Id>>,
    pub faces_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub id: Id,
    pub model: Option<String>,
    pub scripts: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    pub resources: HashMap<Id, ResourceRef>, //TODO model meta file
    pub scripts: HashMap<Id, Script>,

    loaded_scripts: HashMap<String, Id>,
}

impl ResourceManager {
    fn parse_script(&mut self, name: &str, dir: &Path) -> Option<Id> {
        if let Some(script) = self.loaded_scripts.get(name) {
            return Some(script.clone())
        }

        let path = dir.join(name);
        log::debug!("trying to load script: {:?}", path);

        let script: Result<Script, Error> = serde_json::from_str(read_to_string(path).ok()?.as_str());
        match script {
            Ok(script) => {
                let id = script.id.clone();

                self.loaded_scripts.insert(name.to_string(), script.id.clone());
                self.scripts.insert(id.clone(), script);

                Some(id)
            }
            Err(err) => {
                log::warn!("failed to parse script: {}", err);

                None
            }
        }
    }

    fn parse_model(&self, model: Option<String>, working_dir: &Path) -> Option<Model> {
        model
            .map(|v| {
                let path = working_dir.join(v);
                log::debug!("trying to load model: {:?}", path);
                path
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
        resource: Resource,
        dir: &Path,
    ) -> Option<(Id, Option<Model>)> {
        let id = resource.id;

        let model = self.parse_model(resource.model, dir)
            .inspect(|v| {
                log::debug!("loaded model vertices: {}", v.vertices.len());
                log::debug!("loaded model faces: {}", v.faces.len());
            });

        let scripts = resource.scripts.map(|scripts| {
            scripts
                .into_iter()
                .map(|script| {
                    self.parse_script(&script, &dir.join("scripts"))
                })
                .flatten()
                .collect::<Vec<_>>()
        });

        println!("{}, {:?}", id, scripts);

        self.resources
            .insert(id.clone(), ResourceRef { scripts, faces_index: None });

        Some((id, model))
    }
}
