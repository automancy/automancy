use rune::Any;
use serde::{Deserialize, Serialize};

use crate::resource::item::{Item, ItemRaw};
use crate::util::id::Interner;

pub type ItemAmount = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStackRaw {
    pub item: ItemRaw,
    pub amount: ItemAmount,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Any)]
pub struct ItemStack {
    #[rune(get, copy)]
    pub item: Item,
    #[rune(get, copy)]
    pub amount: ItemAmount,
}

impl ItemStackRaw {
    pub fn to_item(self, interner: &mut Interner) -> ItemStack {
        ItemStack {
            item: self.item.to_item(interner),
            amount: self.amount,
        }
    }
}
