pub use chrono;
pub use glam;
pub use gltf;
pub use hex;
pub use hexx;
pub use kira;
pub use log;
pub use slice_group_by;
pub use string_interner;
pub use ttf_parser;

pub mod colors;
pub mod coord;
pub mod id;
pub mod math;
pub mod rendering;
pub mod stack;
pub mod window;
use id::{Id, Interner, SharedStr};
use stack::{ItemAmount, ItemStack};

pub fn parse_map_id_of<T, R: FromIterator<(Id, T)>>(
    v: impl Iterator<Item = (impl AsRef<str>, T)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> R {
    v.flat_map(|(k, v)| Id::parse(k.as_ref(), interner, namespace.as_ref()).zip(Some(v)))
        .collect()
}

pub fn parse_map_v_id<T, R: FromIterator<(T, Id)>>(
    v: impl Iterator<Item = (T, impl AsRef<str>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> R {
    v.flat_map(|(k, v)| Some(k).zip(Id::parse(v.as_ref(), interner, namespace.as_ref())))
        .collect()
}

pub fn parse_map_id_str<R: FromIterator<(Id, SharedStr)>>(
    v: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> R {
    parse_map_id_of(
        v.map(|(k, v)| (k, SharedStr::from(v.as_ref()))),
        interner,
        namespace,
    )
}

pub fn parse_ids<R: FromIterator<Id>>(
    v: impl Iterator<Item = impl AsRef<str>>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> R {
    v.flat_map(|id| Id::parse(id.as_ref(), interner, namespace.as_ref()))
        .collect()
}

pub fn parse_item_stacks<R: FromIterator<ItemStack>>(
    v: impl Iterator<Item = (impl AsRef<str>, impl Into<ItemAmount>)>,
    interner: &mut Interner,
    namespace: Option<impl AsRef<str>>,
) -> R {
    v.flat_map(|(id, amount)| {
        Id::parse(id.as_ref(), interner, namespace.as_ref()).zip(Some(amount.into()))
    })
    .map(|(id, amount)| ItemStack { id, amount })
    .collect()
}

pub fn try_parse_map_id_of<T, R: FromIterator<(Id, T)>>(
    v: impl Iterator<Item = (impl AsRef<str>, T)>,
    interner: &Interner,
) -> R {
    v.flat_map(|(k, v)| Id::try_parse(k.as_ref(), interner).zip(Some(v)))
        .collect()
}

pub fn try_parse_map_v_id<T, R: FromIterator<(T, Id)>>(
    v: impl Iterator<Item = (T, impl AsRef<str>)>,
    interner: &Interner,
) -> R {
    v.flat_map(|(k, v)| Some(k).zip(Id::try_parse(v.as_ref(), interner)))
        .collect()
}

pub fn try_parse_map_id_str<R: FromIterator<(Id, SharedStr)>>(
    v: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    interner: &Interner,
) -> R {
    try_parse_map_id_of(v.map(|(k, v)| (k, SharedStr::from(v.as_ref()))), interner)
}

pub fn try_parse_ids<R: FromIterator<Id>>(
    v: impl Iterator<Item = impl AsRef<str>>,
    interner: &Interner,
) -> R {
    v.flat_map(|id| Id::try_parse(id.as_ref(), interner))
        .collect()
}

pub fn resolve_map_id_of<T, R: FromIterator<(String, T)>>(
    v: impl Iterator<Item = (Id, T)>,
    interner: &Interner,
) -> R {
    v.flat_map(|(k, v)| interner.resolve(k).map(str::to_string).zip(Some(v)))
        .collect()
}

pub fn resolve_map_v_id<T, R: FromIterator<(T, String)>>(
    v: impl Iterator<Item = (T, Id)>,
    interner: &Interner,
) -> R {
    v.flat_map(|(k, v)| Some(k).zip(interner.resolve(v).map(str::to_string)))
        .collect()
}

pub fn resolve_map_id_str<R: FromIterator<(String, String)>>(
    v: impl Iterator<Item = (Id, impl AsRef<str>)>,
    interner: &Interner,
) -> R {
    v.flat_map(|(k, v)| {
        interner
            .resolve(k)
            .map(str::to_string)
            .zip(Some(String::from(v.as_ref())))
    })
    .collect()
}

pub fn resolve_ids<R: FromIterator<String>>(v: impl Iterator<Item = Id>, interner: &Interner) -> R {
    v.flat_map(|id| interner.resolve(id).map(str::to_string))
        .collect()
}
