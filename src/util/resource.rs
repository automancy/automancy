use std::{collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::Path};
use std::fs::{read_dir, read_to_string};

use ply_rs::parser::Parser;
use serde::Deserialize;
use serde_json::Error;

use crate::{
    render::data::{Face, Model, Vertex},
    data::id::{Id},
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
    pub scripts: Option<Vec<Id>>,//TODO model meta file
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    pub ordered_ids: Vec<Id>,
    pub resources: HashMap<Id, ResourceRef>,
    pub scripts: HashMap<Id, Script>,
    pub translates: Translate,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
}

impl ResourceManager {
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

    pub fn load_translates(
        &mut self,
        dir: &Path,
    ) -> Option<()> {
        let translates = dir.join("translates");
        let translates = read_dir(translates).ok()?;

        translates
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .for_each(|translate| {
                // TODO language selection
                if translate.file_stem() == Some(OsStr::new("en_US")) {
                    self.load_translate(&translate);
                }
            });

        Some(())
    }

    pub fn load_scripts(
        &mut self,
        dir: &Path,
    ) -> Option<()> {
        let scripts = dir.join("scripts");
        let scripts = read_dir(scripts).ok()?;

        scripts
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .for_each(|script| {
                self.load_script(&script);
            });

        Some(())
    }

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

        self.resources
            .insert(id.clone(), ResourceRef { scripts: resource.scripts, faces_index: None });

        Some((id, model))
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