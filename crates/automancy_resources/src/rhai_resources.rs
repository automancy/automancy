use automancy_defs::id::{Id, TileId};
use rhai::{Dynamic, Engine};

use crate::RESOURCE_MAN;

pub(crate) fn register_resources(engine: &mut Engine) {
    engine.register_fn("as_script", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .scripts
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tile", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .tiles
            .get(&TileId(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_item", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .items
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tag", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .clone()
            .unwrap()
            .registry
            .tags
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
}
