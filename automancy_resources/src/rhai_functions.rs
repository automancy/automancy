use crate::data::item::{rhai_item_match, rhai_item_matches, rhai_item_stack_matches};
use rhai::Engine;

pub(crate) fn register_functions(engine: &mut Engine) {
    engine.register_fn("item_match", rhai_item_match);
    engine.register_fn("item_matches", rhai_item_matches);
    engine.register_fn("item_matches", rhai_item_stack_matches);
}
