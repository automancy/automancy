use automancy_data::{
    fake_static,
    game::coord::{TileCoord, TileCoordBounds, TileUnit},
    lua::doc::LuaObjectDoc,
};
use automancy_macros::lua_class;

pub struct TileCoordClass {}

/// Check out https://www.redblobgames.com/grids/hexagons/#coordinates-cube for how this coordinate system works.
#[lua_class]
impl TileCoordClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!(
            "{}\n\n{}\n",
            TileCoord::lua_object_doc(),
            TileCoordBounds::lua_object_doc()
        ))
    }

    /// Creates a TileCoord from a `q` component and a `r` component.
    fn new(_lua: &mlua::Lua, q: TileUnit, r: TileUnit) -> mlua::Result<TileCoord> {
        Ok(TileCoord::new(q, r))
    }

    /// The origin point, equals to `TileCoord.new(0, 0)`.
    const ZERO: TileCoord = TileCoord::ZERO;
    /// The 'top right' direction from the origin, equals to `TileCoord.new(0, 1)`.
    const TOP_RIGHT: TileCoord = TileCoord::TOP_RIGHT;
    /// The 'top left' direction from the origin, equals to `TileCoord.new(-1, 1)`.
    const TOP_LEFT: TileCoord = TileCoord::TOP_LEFT;
    /// The 'left' direction from the origin, equals to `TileCoord.new(-1, 0)`.
    const LEFT: TileCoord = TileCoord::LEFT;
    /// The 'bottom left' direction from the origin, equals to `TileCoord.new(0, -1)`.
    const BOTTOM_LEFT: TileCoord = TileCoord::BOTTOM_LEFT;
    /// The 'bottom right' direction from the origin, equals to `TileCoord.new(1, -1)`.
    const BOTTOM_RIGHT: TileCoord = TileCoord::BOTTOM_RIGHT;
    /// The 'right' direction from the origin, equals to `TileCoord.new(1, 0)`.
    const RIGHT: TileCoord = TileCoord::RIGHT;
}
