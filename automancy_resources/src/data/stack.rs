use rhai::INT;

use crate::data::item::Item;

pub type ItemAmount = INT;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub item: Item,
    pub amount: ItemAmount,
}
