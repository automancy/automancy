use std::{collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::Path};
use std::convert::AsRef;
use std::fs::{read_dir, read_to_string};

use ply_rs::parser::Parser;
use serde::Deserialize;
use serde_json::Error;

use crate::{
    render::data::{RawFace, RawModel, Vertex},
    data::id::{Id},
};
use crate::game::script::Script;

pub static JSON_EXT: &str = "json";

#[derive(Debug, Clone)]
pub struct ResourceRef {
    pub scripts: Option<Vec<Id>>,
    pub faces_index: Option<usize>,
    pub resource_t: ResourceType,
}

#[derive(Debug, Default, Clone)]
pub struct Face {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum ResourceType {
    None,
    Model,
    Machine,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub id: Id,
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub resource_t: ResourceType,
    pub id: Id,
    pub model: Option<Id>,
    pub scripts: Option<Vec<Id>>,
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    pub ordered_ids: Vec<Id>,

    pub resources: HashMap<Id, ResourceRef>,
    pub scripts: HashMap<Id, Script>,
    pub translates: Translate,

    pub raw_models: HashMap<Id, RawModel>,
    pub models_referenced: HashMap<Id, Vec<Id>>,
    pub all_faces: Vec<Vec<Face>>,
}

impl ResourceManager {
    fn load_translate(&mut self, file: &Path) -> Option<()> {
        log::debug!("trying to load translate: {:?}", file);

        let translate: Result<Translate, Error> = serde_json::from_str(read_to_string(file).ok()?.as_str());

        match translate {
            Ok(translate) => {
                self.translates = translate;

                Some(())
            }
            Err(err) => {
                log::error!("failed to parse translate: {}", err);

                None
            }
        }
    }

    fn load_script(&mut self, file: &Path) -> Option<()> {
        log::debug!("trying to load script: {:?}", file);

        let script: Result<Script, Error> = serde_json::from_str(read_to_string(file).ok()?.as_str());

        match script {
            Ok(ref script) => {
                self.scripts.insert(script.id.clone(), script.clone());

                Some(())
            }
            Err(err) => {
                log::error!("failed to parse script: {}", err);

                None
            }
        }
    }

    fn load_model(&mut self, file: &Path) -> Option<()> {
        log::debug!("trying to load model: {:?}", file);

        let model: Result<Model, Error> = serde_json::from_str(read_to_string(file).ok()?.as_str());

        match model {
            Ok(ref model) => {
                let file = file.parent().unwrap().join("files").join(model.file.as_str());
                log::debug!("trying to load model file: {:?}", file);

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
                                .read_payload_for_element(&mut read, &element, &header)
                                .ok();
                        }
                        "face" => {
                            faces = face_parser
                                .read_payload_for_element(&mut read, &element, &header)
                                .ok();
                        }
                        _ => (),
                    }
                }

                let raw_model = vertices
                    .zip(faces)
                    .map(|(vertices, faces)| RawModel::new(vertices, faces))?;

                self.raw_models.insert(model.id.clone(), raw_model);

                Some(())
            }
            Err(err) => {
                log::error!("failed to parse model: {}", err);

                None
            }
        }
    }

    pub fn load_translates(&mut self, dir: &Path) -> Option<()> {
        let translates = dir.join("translates");
        let translates = read_dir(translates).ok()?;

        translates
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|translate| {
                // TODO language selection
                if translate.file_stem() == Some(OsStr::new("en_US")) {
                    self.load_translate(&translate);
                }
            });

        Some(())
    }

    pub fn load_scripts(&mut self, dir: &Path) -> Option<()> {
        let scripts = dir.join("scripts");
        let scripts = read_dir(scripts).ok()?;

        scripts
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|script| {
                self.load_script(&script);
            });

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

    pub fn register_resource(&mut self, resource: Resource) {
        let id = resource.id;

        if let Some(model) = resource.model {
            let references = self.models_referenced.entry(model).or_insert_with(Vec::default);

            references.push(id.clone());
        }

        self.resources.insert(
            id,
            ResourceRef {
                scripts: resource.scripts,
                faces_index: None,
                resource_t: resource.resource_t
            }
        );
    }
}

impl ResourceManager {
    pub fn item_name(&self, id: &Id) -> String {
        match self.translates.items.get(id) {
            Some(name) => { name.to_owned() }
            None => { id.to_string() }
        }
    }

    pub fn tile_name(&self, id: &Id) -> String {
        match self.translates.tiles.get(id) {
            Some(name) => { name.to_owned() }
            None => { id.to_string() }
        }
    }
}