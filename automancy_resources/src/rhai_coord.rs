use automancy_defs::coord::TileCoord;
use rhai::{Engine, Module};
use std::ops::{Add, Neg, Sub};

pub(crate) fn register_coord_stuff(engine: &mut Engine) {
    let mut module = Module::new();

    module
        .set_var("ZERO", TileCoord::ZERO)
        .set_var("TOP_RIGHT", TileCoord::TOP_RIGHT)
        .set_var("RIGHT", TileCoord::RIGHT)
        .set_var("BOTTOM_RIGHT", TileCoord::BOTTOM_RIGHT)
        .set_var("BOTTOM_LEFT", TileCoord::BOTTOM_LEFT)
        .set_var("LEFT", TileCoord::LEFT)
        .set_var("TOP_LEFT", TileCoord::TOP_LEFT);

    engine.register_static_module("TileCoord", module.into());

    engine
        .register_type_with_name::<TileCoord>("TileCoord")
        .register_fn("to_string", |v: TileCoord| v.to_string())
        .register_iterator::<Vec<TileCoord>>()
        .register_fn("TileCoord", TileCoord::new)
        .register_fn("rotate_left", |n: TileCoord| {
            TileCoord::from(n.counter_clockwise())
        })
        .register_fn("rotate_right", |n: TileCoord| {
            TileCoord::from(n.clockwise())
        })
        .register_get("q", |v: &mut TileCoord| v.x)
        .register_get("r", |v: &mut TileCoord| v.y)
        .register_fn("+", TileCoord::add)
        .register_fn("-", TileCoord::sub)
        .register_fn("-", TileCoord::neg)
        .register_fn("==", |a: TileCoord, b: TileCoord| a == b)
        .register_fn("!=", |a: TileCoord, b: TileCoord| a != b);
}
