use std::collections::BTreeMap;

use automancy_data::{
    game::coord::{RadialTileBounds, TileCoord, TileCoordBounds, TileUnit},
    id::{Id, ModelId},
    math::Matrix4,
};
use rhai::{Dynamic, Engine, Module};

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
        .register_fn("neighbors", |v: TileCoord| -> Dynamic { Dynamic::from_iter(v.neighbors()) })
        .register_fn("TileCoord", TileCoord::new)
        .register_fn("rotate_ccw", |v: TileCoord| -> TileCoord { v.ccw() })
        .register_fn("rotate_cw", |v: TileCoord| -> TileCoord { v.cw() })
        .register_fn("as_translation", |v: TileCoord| -> Matrix4 { v.as_translation() })
        .register_fn("as_rotation_z", |v: TileCoord| -> Matrix4 {
            Matrix4::rotation_z(TileCoord::as_degrees(v).to_radians())
        })
        .register_get("q", |v: &mut TileCoord| -> TileUnit { v.q })
        .register_get("r", |v: &mut TileCoord| -> TileUnit { v.r })
        .register_fn("+", TileCoord::add)
        .register_fn("-", TileCoord::sub)
        .register_fn("-", TileCoord::neg)
        .register_fn("==", |a: TileCoord, b: TileCoord| a == b)
        .register_fn("!=", |a: TileCoord, b: TileCoord| a != b);

    engine
        .register_type_with_name::<TileCoordBounds>("TileCoordBounds")
        .register_iterator::<TileCoordBounds>()
        .register_fn("RadialTileBounds", TileCoordBounds::radial)
        .register_fn("RadialTileBounds", |v: Vec<TileCoord>| -> TileCoordBounds {
            TileCoordBounds::Radial(RadialTileBounds::from_iter(v))
        })
        .register_fn("RadialTileBounds", |v: Vec<(TileCoord, Id)>| -> TileCoordBounds {
            TileCoordBounds::Radial(RadialTileBounds::from_iter(v.into_iter().map(|v| v.0)))
        })
        .register_fn("RadialTileBounds", |v: BTreeMap<TileCoord, Id>| -> TileCoordBounds {
            TileCoordBounds::Radial(RadialTileBounds::from_iter(v.into_iter().map(|v| v.0)))
        })
        .register_fn("RadialTileBounds", |v: BTreeMap<TileCoord, ModelId>| -> TileCoordBounds {
            TileCoordBounds::Radial(RadialTileBounds::from_iter(v.into_iter().map(|v| v.0)))
        })
        .register_fn("contains", |v: &mut TileCoordBounds, coord: TileCoord| -> bool {
            v.contains(coord)
        });
}
