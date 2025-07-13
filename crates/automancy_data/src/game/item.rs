use crate::{id::Id, math::Int};

pub type ItemAmount = Int;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Id,
    pub amount: ItemAmount,
}

pub mod deserialize {
    use serde_tuple::Deserialize_tuple;

    use crate::{game::item::ItemAmount, id::deserialize::StrId};

    #[derive(Debug, PartialEq, Eq, Deserialize_tuple)]
    pub struct ItemStackStr {
        pub id: StrId,
        pub amount: ItemAmount,
    }
}
