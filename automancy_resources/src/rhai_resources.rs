use crate::RESOURCE_MAN;
use automancy_defs::id::Id;
use rhai::{Dynamic, Engine, INT};

pub(crate) fn register_resources(engine: &mut Engine) {
    engine.register_fn("as_script", |id: INT| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .scripts
            .get(&Id::from(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tile", |id: INT| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .tiles
            .get(&Id::from(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_item", |id: INT| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .items
            .get(&Id::from(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tag", |id: INT| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .clone()
            .unwrap()
            .registry
            .tags
            .get(&Id::from(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
}
