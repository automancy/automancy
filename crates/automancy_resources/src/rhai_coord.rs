use automancy_defs::{
    coord::{TileBounds, TileCoord, TileUnit},
    math::tile_direction_to_angle,
};
use automancy_defs::{id::Id, math::Matrix4};
use hashbrown::HashMap;
use rhai::{Dynamic, Engine, Module};
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
        .register_fn("to_string", |v: TileCoord| -> String { v.to_string() })
        .register_fn("neighbors", |v: TileCoord| -> Dynamic {
            Dynamic::from_iter(v.neighbors())
        })
        .register_fn("TileCoord", TileCoord::new)
        .register_fn("rotate_left", |v: TileCoord| -> TileCoord {
            TileCoord::from(v.counter_clockwise())
        })
        .register_fn("rotate_right", |v: TileCoord| -> TileCoord {
            TileCoord::from(v.clockwise())
        })
        .register_fn("as_translation", |v: TileCoord| -> Matrix4 {
            v.as_translation()
        })
        .register_fn("as_rotation_z", |v: TileCoord| -> Matrix4 {
            let Some(deg) = tile_direction_to_angle(v) else {
                return Matrix4::IDENTITY;
            };

            Matrix4::from_rotation_z(deg.to_radians())
        })
        .register_get("q", |v: &mut TileCoord| -> TileUnit { v.x })
        .register_get("r", |v: &mut TileCoord| -> TileUnit { v.y })
        .register_fn("+", TileCoord::add)
        .register_fn("-", TileCoord::sub)
        .register_fn("-", TileCoord::neg)
        .register_fn("==", |a: TileCoord, b: TileCoord| a == b)
        .register_fn("!=", |a: TileCoord, b: TileCoord| a != b);

    engine
        .register_type_with_name::<TileBounds>("TileBounds")
        .register_iterator::<TileBounds>()
        .register_fn("TileBounds", TileBounds::new)
        .register_fn("TileBounds", |v: Vec<TileCoord>| -> TileBounds {
            TileBounds::from_iter(v)
        })
        .register_fn("TileBounds", |v: Vec<(TileCoord, Id)>| -> TileBounds {
            TileBounds::from_iter(v.into_iter().map(|v| v.0))
        })
        .register_fn("TileBounds", |v: HashMap<TileCoord, Id>| -> TileBounds {
            TileBounds::from_iter(v.into_iter().map(|v| v.0))
        })
        .register_fn("contains", |v: &mut TileBounds, coord: TileCoord| -> bool {
            v.is_in_bounds(*coord)
        });
}
