use std::{fs, io};

use ply_rs::parser::Parser;

use crate::game::render::data::{Face, Model, Vertex};

#[derive(Clone, Debug)]
pub struct Resource {
    registry_index: Option<usize>,

    pub mesh: Model,
}

impl Resource {
    pub fn register(&mut self, index: usize) {
        self.registry_index = Some(index);
    }
}

pub async fn load_resource(path: &str) -> Resource {
    log::info!("loading resource at {}", path);

    let file = fs::File::open(path).unwrap();
    let mut file = io::BufReader::new(file);

    let vertex_parser = Parser::<Vertex>::new();
    let face_parser = Parser::<Face>::new();

    let header = vertex_parser.read_header(&mut file).unwrap();

    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for (_, element) in &header.elements {
        match element.name.as_ref() {
            "vertex" => {
                vertices = vertex_parser
                    .read_payload_for_element(&mut file, &element, &header)
                    .unwrap();
            }
            "face" => {
                faces = face_parser
                    .read_payload_for_element(&mut file, &element, &header)
                    .unwrap();
            }
            _ => {}
        }
    }

    let mesh = Model { vertices, faces };

    Resource {
        registry_index: None,
        mesh,
    }
}
