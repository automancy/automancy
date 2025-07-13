use rhai::INT;

use crate::id::Id;

pub type ItemAmount = INT;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub id: Id,
    pub amount: ItemAmount,
}
