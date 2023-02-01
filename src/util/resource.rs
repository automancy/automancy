use std::convert::AsRef;
use std::fs::{read_dir, read_to_string};
use std::{collections::HashMap, ffi::OsStr, fs::File, io::BufReader, path::Path};

use ply_rs::parser::Parser;
use serde::Deserialize;
use serde_json::Error;

use crate::game::script::{Instructions, Script, ScriptRaw};
use crate::render::data::{ModelRaw, RawFace, Vertex};
use crate::util::id::{Id, IdRaw, Interner};

pub static JSON_EXT: &str = "json";

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub scripts: Option<Vec<Id>>,
    pub faces_index: Option<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct Face {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", content = "param")]
pub enum ResourceType {
    None,
    Void,
    Model,
    Machine(IdRaw),
    Transfer(IdRaw),
}

#[derive(Debug, Default, Clone)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TranslateRaw {
    pub items: HashMap<IdRaw, String>,
    pub tiles: HashMap<IdRaw, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub id: IdRaw,
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceRaw {
    pub resource_type: ResourceType,
    pub id: IdRaw,
    pub model: Option<IdRaw>,
    pub scripts: Option<Vec<IdRaw>>,
}

#[derive(Debug)]
pub struct ResourceManager {
    pub interner: Interner,
    pub none: Id,

    pub ordered_ids: Vec<Id>,

    pub resources: HashMap<Id, Resource>,
    pub scripts: HashMap<Id, Script>,
    pub translates: Translate,

    pub raw_models: HashMap<Id, ModelRaw>,
    pub models_referenced: HashMap<Id, Vec<Id>>,
    pub all_faces: Vec<Vec<Face>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let none = IdRaw::NONE.to_id(&mut interner);

        Self {
            interner,
            none,

            ordered_ids: vec![],

            resources: Default::default(),
            scripts: Default::default(),
            translates: Default::default(),

            raw_models: Default::default(),
            models_referenced: Default::default(),
            all_faces: vec![],
        }
    }
}

impl ResourceManager {
    fn load_translate(&mut self, file: &Path) -> Option<()> {
        log::debug!("trying to load translate: {:?}", file);

        let translate: Result<TranslateRaw, Error> =
            serde_json::from_str(read_to_string(file).ok()?.as_str());

        match translate {
            Ok(translate) => {
                let items = translate
                    .items
                    .into_iter()
                    .map(|(id, str)| (id.to_id(&mut self.interner), str))
                    .collect();
                let tiles = translate
                    .tiles
                    .into_iter()
                    .map(|(id, str)| (id.to_id(&mut self.interner), str))
                    .collect();

                self.translates = Translate { items, tiles };

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

        let script: Result<ScriptRaw, Error> =
            serde_json::from_str(read_to_string(file).ok()?.as_str());

        match script {
            Ok(ref script) => {
                let id = script.id.to_id(&mut self.interner);

                let instructions = Instructions {
                    input: script
                        .instructions
                        .input
                        .as_ref()
                        .map(|v| v.to_item(&mut self.interner)),
                    output: script
                        .instructions
                        .output
                        .as_ref()
                        .map(|v| v.to_item(&mut self.interner)),
                };

                let script = Script { id, instructions };

                self.scripts.insert(id, script);

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
                let file = file
                    .parent()
                    .unwrap()
                    .join("files")
                    .join(model.file.as_str());
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
                    .map(|(vertices, faces)| ModelRaw::new(vertices, faces))?;

                self.raw_models
                    .insert(model.id.to_id(&mut self.interner), raw_model);

                Some(())
            }
            Err(err) => {
                log::error!("failed to parse model: {}", err);

                None
            }
        }
    }

    fn load_tile(&mut self, file: &Path) -> Option<()> {
        let extension = file.extension().and_then(OsStr::to_str);

        if let Some("json") = extension {
            log::info!("loading resource at {:?}", file);

            let resource: ResourceRaw =
                serde_json::from_str(&read_to_string(&file).unwrap()).unwrap();

            self.register_resource(resource);
        }
        Some(())
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

    pub fn load_tiles(&mut self, dir: &Path) -> Option<()> {
        let tiles = dir.join("tiles");
        let tiles = read_dir(tiles).ok()?;

        tiles
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|tile| {
                self.load_tile(&tile);
            });

        Some(())
    }
    pub fn register_resource(&mut self, resource: ResourceRaw) {
        let id = resource.id.to_id(&mut self.interner);

        if let Some(model) = resource.model {
            let references = self
                .models_referenced
                .entry(model.to_id(&mut self.interner))
                .or_insert_with(Vec::default);

            references.push(id);
        }

        let scripts = resource.scripts.map(|v| {
            v.into_iter()
                .map(|id| id.to_id(&mut self.interner))
                .collect()
        });

        let resource_type = resource.resource_type;

        self.resources.insert(
            id,
            Resource {
                resource_type,
                scripts,
                faces_index: None,
            },
        );
    }
}

impl ResourceManager {
    pub fn item_name(&self, id: &Id) -> String {
        match self.translates.items.get(id) {
            Some(name) => name.to_owned(),
            None => "<unnamed>".to_string(),
        }
    }

    pub fn tile_name(&self, id: &Id) -> String {
        match self.translates.tiles.get(id) {
            Some(name) => name.to_owned(),
            None => "<unnamed>".to_string(),
        }
    }
}
