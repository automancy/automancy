use crate::data::id::Id;
use serde::Deserialize;

pub type ItemAmount = u64;

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: Id,
    pub amount: ItemAmount,
}