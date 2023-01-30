use serde::Deserialize;

use crate::data::id::{Id, IdRaw};
use crate::game::item::{Item, ItemRaw};

#[derive(Debug, Clone, Copy)]
pub struct Script {
    pub id: Id,
    pub script_t: Id,
    pub instructions: Instructions,
}

#[derive(Debug, Clone, Copy)]
pub struct Instructions {
    pub input: Option<Item>,
    pub output: Option<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptRaw {
    pub id: IdRaw,
    pub script_t: IdRaw,
    pub instructions: InstructionsRaw,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstructionsRaw {
    pub input: Option<ItemRaw>,
    pub output: Option<ItemRaw>,
}