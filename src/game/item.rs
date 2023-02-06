use crate::resource::item::{Item, ItemRaw};
use serde::{Deserialize, Serialize};

use crate::util::id::Interner;

pub type ItemAmount = u64;

#[derive(Debug, Clone, Copy)]
pub struct ItemStack {
    pub item: Item,
    pub amount: ItemAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStackRaw {
    pub item: ItemRaw,
    pub amount: ItemAmount,
}

impl ItemStackRaw {
    pub fn to_item(&self, interner: &mut Interner) -> ItemStack {
        ItemStack {
            item: Item {
                id: self.item.id.to_id(interner),
            },
            amount: self.amount,
        }
    }
}
