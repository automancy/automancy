use std::fmt::{Debug, Formatter};
use std::{collections::HashMap, fmt};

use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackHandle;
use serde::Deserialize;

use crate::render::data::{Model, RawFace, Vertex};
use crate::render::gui::GuiId;
use crate::resource::functions::Function;
use crate::resource::model::Face;
use crate::resource::script::Script;
use crate::resource::tile::Tile;
use crate::resource::translate::Translate;
use crate::util::id::{Id, IdRaw, Interner};

pub mod audio;
pub mod functions;
pub mod model;
pub mod script;
pub mod tile;
pub mod translate;

pub static JSON_EXT: &str = "json";
pub static OGG_EXT: &str = "ogg";
pub static RESOURCES_FOLDER: &str = "resources";

pub struct ResourceManager {
    pub interner: Interner,
    pub none: Id,
    pub track: TrackHandle,

    pub ordered_ids: Vec<Id>,

    pub tiles: HashMap<Id, Tile>,
    pub scripts: HashMap<Id, Script>,
    pub translates: Translate,
    pub audio: HashMap<String, StaticSoundData>,
    pub functions: HashMap<Id, Function>,

    pub faces: HashMap<Id, Face>,

    pub all_vertices: Vec<Vertex>,
    pub raw_models: HashMap<Id, Model>,
    pub raw_faces: Vec<RawFace>,

    pub gui_ids: GuiId,
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
        let gui_ids = GuiId::new(&mut interner);
        Self {
            interner,
            none,
            track,

            ordered_ids: vec![],

            tiles: Default::default(),
            scripts: Default::default(),
            translates: Default::default(),
            audio: Default::default(),
            functions: Default::default(),

            faces: Default::default(),

            all_vertices: Default::default(),
            raw_models: Default::default(),
            raw_faces: Default::default(),

            gui_ids,
        }
    }
}

impl ResourceManager {}

impl ResourceManager {}
