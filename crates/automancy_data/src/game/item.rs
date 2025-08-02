use crate::{id::Id, math::Int};

pub type ItemAmount = Int;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ItemStack {
    pub id: Id,
    pub amount: ItemAmount,
}
