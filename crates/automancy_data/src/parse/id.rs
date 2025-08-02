use crate::{
    game::item::{ItemAmount, ItemStack},
    id::{Id, Interner},
};

pub fn parse_ids(
    iter: impl Iterator<Item = impl AsRef<str>>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> impl Iterator<Item = Id> {
    iter.flat_map(move |id| Id::parse(id.as_ref(), interner, namespace.as_ref()))
}

pub fn try_parse_ids(
    iter: impl Iterator<Item = impl AsRef<str>>,
    interner: &Interner,
) -> impl Iterator<Item = Id> {
    iter.flat_map(move |id| Id::try_parse(id.as_ref(), interner))
}

pub fn resolve_ids(
    iter: impl Iterator<Item = Id>,
    interner: &Interner,
) -> impl Iterator<Item = String> {
    iter.flat_map(|id| interner.resolve(id).map(str::to_string))
}

pub fn parse_map_id_item<Item>(
    iter: impl Iterator<Item = (impl AsRef<str>, Item)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> impl Iterator<Item = (Id, Item)> {
    iter.flat_map(move |(k, v)| Id::parse(k.as_ref(), interner, namespace.as_ref()).zip(Some(v)))
}

pub fn try_parse_map_id_item<Item>(
    iter: impl Iterator<Item = (impl AsRef<str>, Item)>,
    interner: &Interner,
) -> impl Iterator<Item = (Id, Item)> {
    iter.flat_map(move |(k, v)| Id::try_parse(k.as_ref(), interner).zip(Some(v)))
}

pub fn parse_map_item_id<Item>(
    iter: impl Iterator<Item = (Item, impl AsRef<str>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> impl Iterator<Item = (Item, Id)> {
    iter.flat_map(move |(k, v)| Some(k).zip(Id::parse(v.as_ref(), interner, namespace.as_ref())))
}

pub fn try_parse_map_item_id<Item>(
    iter: impl Iterator<Item = (Item, impl AsRef<str>)>,
    interner: &Interner,
) -> impl Iterator<Item = (Item, Id)> {
    iter.flat_map(move |(k, v)| Some(k).zip(Id::try_parse(v.as_ref(), interner)))
}

pub fn parse_map_id_str(
    iter: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> impl Iterator<Item = (Id, String)> {
    parse_map_id_item(
        iter.map(|(k, v)| (k, v.as_ref().to_string())),
        interner,
        namespace,
    )
}

pub fn try_parse_map_id_str(
    iter: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    interner: &Interner,
) -> impl Iterator<Item = (Id, String)> {
    try_parse_map_id_item(iter.map(|(k, v)| (k, v.as_ref().to_string())), interner)
}

pub fn parse_item_stacks(
    iter: impl Iterator<Item = (impl AsRef<str>, impl Into<ItemAmount>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> impl Iterator<Item = ItemStack> {
    iter.flat_map(move |(id, amount)| {
        Id::parse(id.as_ref(), interner, namespace.as_ref()).zip(Some(amount.into()))
    })
    .map(|(id, amount)| ItemStack { id, amount })
}

pub fn try_parse_item_stacks(
    iter: impl Iterator<Item = (impl AsRef<str>, impl Into<ItemAmount>)>,
    interner: &Interner,
) -> impl Iterator<Item = ItemStack> {
    iter.flat_map(move |(id, amount)| Id::try_parse(id.as_ref(), interner).zip(Some(amount.into())))
        .map(|(id, amount)| ItemStack { id, amount })
}

pub fn resolve_map_id_item<Item>(
    iter: impl Iterator<Item = (Id, Item)>,
    interner: &Interner,
) -> impl Iterator<Item = (String, Item)> {
    iter.flat_map(|(k, v)| interner.resolve(k).map(str::to_string).zip(Some(v)))
}

pub fn resolve_map_item_id<Item>(
    iter: impl Iterator<Item = (Item, Id)>,
    interner: &Interner,
) -> impl Iterator<Item = (Item, String)> {
    iter.flat_map(|(k, v)| Some(k).zip(interner.resolve(v).map(str::to_string)))
}

pub fn resolve_map_id_str(
    iter: impl Iterator<Item = (Id, impl AsRef<str>)>,
    interner: &Interner,
) -> impl Iterator<Item = (String, String)> {
    iter.flat_map(|(k, v)| {
        interner
            .resolve(k)
            .map(str::to_string)
            .zip(Some(String::from(v.as_ref())))
    })
}
