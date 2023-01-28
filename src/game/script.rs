use crate::data::id::Id;
use serde::Deserialize;
use crate::game::item::Item;

#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub id: Id,
    pub script_t: Id,
    pub instructions: Instructions,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Instructions {
    pub input: Item,
    pub output: Item,
}