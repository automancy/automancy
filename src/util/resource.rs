use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::track::TrackHandle;
use std::convert::AsRef;
use std::fmt::{Debug, Formatter};
use std::fs::{read_dir, read_to_string};
use std::{collections::HashMap, ffi::OsStr, fmt, fs::File, io::BufReader, path::Path};

use ply_rs::parser::Parser;
use serde::Deserialize;

use crate::game::script::{Instructions, Script, ScriptRaw};
use crate::render::data::{Model, RawFace, Vertex};
use crate::util::id::{Id, IdRaw, Interner};

pub static JSON_EXT: &str = "json";

#[derive(Debug, Default, Clone)]
pub struct Face {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TranslateRaw {
    pub items: HashMap<IdRaw, String>,
    pub tiles: HashMap<IdRaw, String>,
}

#[derive(Debug, Default, Clone)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelRaw {
    pub id: IdRaw,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", content = "param")]
pub enum TileType {
    Empty,
    Void,
    Model,
    Machine(IdRaw),
    Transfer(IdRaw),
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub tile_type: TileType,
    pub scripts: Option<Vec<Id>>,
    pub faces_indices: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileRaw {
    pub tile_type: TileType,
    pub id: IdRaw,
    pub models: Option<Vec<IdRaw>>,
    pub scripts: Option<Vec<IdRaw>>,
}

pub struct ResourceManager {
    pub interner: Interner,
    pub none: Id,
    pub track: TrackHandle,

    pub ordered_ids: Vec<Id>,

    pub tiles: HashMap<Id, Tile>,
    pub scripts: HashMap<Id, Script>,
    pub translates: Translate,
    pub audio: HashMap<String, StaticSoundData>,

    pub raw_models: HashMap<Id, Model>,
    pub models_referenced: HashMap<Id, Vec<Id>>,

    pub all_faces: Vec<Vec<Face>>,
    pub all_vertices: Vec<Vertex>,
    pub all_raw_faces: Vec<Vec<RawFace>>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<resource manager>")
    }
}

impl ResourceManager {
    pub fn new(track: TrackHandle) -> Self {
        let mut interner = Interner::new();
        let none = IdRaw::NONE.to_id(&mut interner);

        Self {
            interner,
            none,
            track,

            ordered_ids: vec![],

            tiles: Default::default(),
            scripts: Default::default(),
            translates: Default::default(),
            audio: Default::default(),

            raw_models: Default::default(),
            models_referenced: Default::default(),

            all_faces: vec![],
            all_vertices: vec![],
            all_raw_faces: vec![],
        }
    }
}

impl ResourceManager {
    fn load_translate(&mut self, file: &Path) -> Option<()> {
        log::info!("loading translate at: {:?}", file);

        let translate: TranslateRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

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

    fn load_script(&mut self, file: &Path) -> Option<()> {
        log::info!("loading script at: {:?}", file);

        let script: ScriptRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

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

    fn load_model(&mut self, file: &Path) -> Option<()> {
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

        self.raw_models
            .insert(model.id.to_id(&mut self.interner), raw_model);

        Some(())
    }

    fn load_tile(&mut self, file: &Path) -> Option<()> {
        log::info!("loading tile at {:?}", file);

        let tile: TileRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let id = tile.id.to_id(&mut self.interner);

        if let Some(models) = tile.models {
            for model in models {
                let references = self
                    .models_referenced
                    .entry(model.to_id(&mut self.interner))
                    .or_insert_with(Vec::default);
                references.push(id);
            }
        }

        let scripts = tile.scripts.map(|v| {
            v.into_iter()
                .map(|id| id.to_id(&mut self.interner))
                .collect()
        });

        let tile_type = tile.tile_type;

        self.tiles.insert(
            id,
            Tile {
                tile_type,
                scripts,
                faces_indices: vec![],
            },
        );

        Some(())
    }

    pub fn load_audio(&mut self, dir: &Path) -> Option<()> {
        let audio = dir.join("audio");
        let audio = read_dir(audio).ok()?;

        audio
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .for_each(|file| {
                log::info!("loading audio at {:?}", file);

                if let Ok(audio) = StaticSoundData::from_file(
                    file.clone(),
                    StaticSoundSettings::default().track(&self.track),
                ) {
                    self.audio.insert(
                        file.file_stem().unwrap().to_str().unwrap().to_string(),
                        audio,
                    );
                }
            });

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
}

impl ResourceManager {
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

        let (all_raw_faces, all_faces): (Vec<_>, Vec<_>) = raw_faces // TODO we can just draw 3 indices a bunch of times
            .into_iter()
            .enumerate()
            .map(|(i, (id, raw_faces))| {
                if let Some(references) = self.models_referenced.get(id) {
                    references.iter().for_each(|id| {
                        if let Some(resource) = self.tiles.get_mut(id) {
                            resource.faces_indices.push(i);
                        }
                    });
                }

                let faces = raw_faces
                    .iter()
                    .map(|face| {
                        let size = face.indices.len() as u32;
                        let offset = offset_count;

                        offset_count += size;

                        Face { size, offset }
                    })
                    .collect::<Vec<_>>();

                let raw_faces = raw_faces
                    .into_iter()
                    .map(|face| face.index_offset(index_offsets[i] as u32))
                    .collect::<Vec<_>>();

                (raw_faces, faces)
            })
            .unzip();

        /*
        log::debug!("combined_vertices: {:?}", combined_vertices);
        log::debug!("all_raw_faces: {:?}", all_raw_faces);
        log::debug!("all_faces: {:?}", all_faces);
         */

        self.all_faces = all_faces;
        self.all_vertices = all_vertices;
        self.all_raw_faces = all_raw_faces;
    }
}

impl ResourceManager {
    pub fn item_name(&self, id: &Id) -> String {
        match self.translates.items.get(id) {
            Some(name) => name.to_owned(),
            None => "<unnamed>".to_string(),
        }
    }

    pub fn try_item_name(&self, id: &Option<Id>) -> String {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            "<none>".to_owned()
        }
    }

    pub fn tile_name(&self, id: &Id) -> String {
        match self.translates.tiles.get(id) {
            Some(name) => name.to_owned(),
            None => "<unnamed>".to_string(),
        }
    }

    pub fn try_tile_name(&self, id: &Option<Id>) -> String {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            "<none>".to_owned()
        }
    }
}
