use crate::data::item::Item;

pub type ItemAmount = i32;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub item: Item,
    pub amount: ItemAmount,
}
