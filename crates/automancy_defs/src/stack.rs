use crate::id::Id;
use rhai::INT;

pub type ItemAmount = INT;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub id: Id,
    pub amount: ItemAmount,
}
