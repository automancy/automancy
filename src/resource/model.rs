use crate::render::data::{Model, RawFace, Vertex};
use crate::resource::JSON_EXT;
use crate::resource::{LoadResource, ResourceManager};
use crate::util::id::IdRaw;
use ply_rs::parser::Parser;
use serde::Deserialize;
use std::any::Any;
use std::ffi::OsStr;
use std::fs::{read_to_string, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

impl LoadResource<Model> for ResourceManager {
    fn load(resource_man: &mut ResourceManager, file: &Path) -> Option<()> {
        log::info!("loading model at: {:?}", file);

        let model: ModelRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let file = file
            .parent()
            .unwrap()
            .join("files")
            .join(model.file.as_str());

        log::info!("loading model file at: {:?}", file);

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

        resource_man
            .raw_models
            .insert(model.id.to_id(&mut resource_man.interner), raw_model);

        Some(())
    }

    const DIR: String = String::from("models");
    const FILTER: dyn FnMut(&PathBuf) -> bool = (|v| v.extension() == Some(OsStr::new(JSON_EXT)));
}
