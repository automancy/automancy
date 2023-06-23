use crate::data::item::Item;

pub type ItemAmount = u64;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub item: Item,
    pub amount: ItemAmount,
}
