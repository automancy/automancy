use core::fmt;

use automancy_data::game::coord::{
    TileCoord, TileUnit,
    lua::{
        doc::{TILE_COORD, TILE_COORD_BOUNDS, TILE_UNIT, TYPE_DOC},
        fields::*,
        singleton::*,
        types::*,
    },
};

use crate::scripting_lua;

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    fmt::write(
        type_def,
        const_format_args!(
            "{TILE_UNIT}

{TILE_COORD_BOUNDS}

{TILE_COORD}
"
        ),
    )
    .unwrap();

    let module = lua.create_table()?;

    module
        .raw_set(
            NEW,
            lua.create_function(|_lua, (q, r): (TileUnit, TileUnit)| Ok(TileCoord::new(q, r)))?,
        )
        .unwrap();
    module.raw_set(ZERO, TileCoord::ZERO)?;
    module.raw_set(TOP_RIGHT, TileCoord::TOP_RIGHT)?;
    module.raw_set(TOP_LEFT, TileCoord::TOP_LEFT)?;
    module.raw_set(LEFT, TileCoord::LEFT)?;
    module.raw_set(BOTTOM_LEFT, TileCoord::BOTTOM_LEFT)?;
    module.raw_set(BOTTOM_RIGHT, TileCoord::BOTTOM_RIGHT)?;
    module.raw_set(RIGHT, TileCoord::RIGHT)?;

    fmt::write(
        type_def,
        const_format_args!(
            "
---@class (exact) {TileCoord}Singleton
---@field {NEW} fun({Q}: {TileUnit}, {R}: {TileUnit}): {TileCoord} Creates a {TileCoord} from a `q` component and a `r` component.
---@field {ZERO} {TileCoord} The origin point, equals to `{TileCoord}.{NEW}(0, 0)`.
---@field {TOP_RIGHT} {TileCoord} The 'top right' direction from the origin, equals to `{TileCoord}.{NEW}(0, 1)`.
---@field {TOP_LEFT} {TileCoord} The 'top left' direction from the origin, equals to `{TileCoord}.{NEW}(-1, 1)`.
---@field {LEFT} {TileCoord} The 'left' direction from the origin, equals to `{TileCoord}.{NEW}(-1, 0)`.
---@field {BOTTOM_LEFT} {TileCoord} The 'bottom left' direction from the origin, equals to `{TileCoord}.{NEW}(0, -1)`.
---@field {BOTTOM_RIGHT} {TileCoord} The 'bottom right' direction from the origin, equals to `{TileCoord}.{NEW}(1, -1)`.
---@field {RIGHT} {TileCoord} The 'right' direction from the origin, equals to `{TileCoord}.{NEW}(1, 0)`.
---{TYPE_DOC}
local {TileCoord} = {{}}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(TileCoord, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {TileCoord}
"
        ),
    )
    .unwrap();

    Ok(())
}
