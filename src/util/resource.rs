use std::{fs::File, io::BufReader, path::PathBuf};

use json::JsonValue;
use ply_rs::parser::Parser;

use crate::game::{
    data::id::Id,
    render::data::{Face, Model, Vertex},
};

#[derive(Clone, Debug)]
pub struct Resource {
    registry_index: Option<usize>,

    pub id: Id,
    pub model: Option<Model>,
}

impl Resource {
    pub fn register(&mut self, index: usize) {
        self.registry_index = Some(index);
    }
}

// TODO do not panic
pub async fn load_resource(json: JsonValue, path: &PathBuf) -> Resource {
    let parent = path.parent().unwrap();

    let id = Id::from_str(json["id"].as_str().expect("found no id"));

    let model;

    if let Some(model_path) = json["model"].as_str().and_then(|m| Some(parent.join(m))) {
        let model_file = File::open(model_path).unwrap();
        let mut model_reader = BufReader::new(model_file);

        let vertex_parser = Parser::<Vertex>::new();
        let face_parser = Parser::<Face>::new();

        let header = vertex_parser.read_header(&mut model_reader).unwrap();

        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        for (_, element) in &header.elements {
            match element.name.as_ref() {
                "vertex" => {
                    vertices = vertex_parser
                        .read_payload_for_element(&mut model_reader, &element, &header)
                        .unwrap();
                }
                "face" => {
                    faces = face_parser
                        .read_payload_for_element(&mut model_reader, &element, &header)
                        .unwrap();
                }
                _ => {}
            }
        }

        model = Some(Model { vertices, faces });
    } else {
        model = None;
    }

    Resource {
        registry_index: None,
        id,
        model,
    }
}
