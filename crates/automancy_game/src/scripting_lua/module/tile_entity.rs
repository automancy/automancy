use core::fmt;

use crate::{
    actor::tile_entity::lua::{doc::*, singleton::NEW, types::TileEntity},
    scripting_lua,
};

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    fmt::write(
        type_def,
        const_format_args!(
            "{TILE_DEFINITION}

{TILE_ENTITY}"
        ),
    )
    .unwrap();

    let module = lua.create_table()?;

    module.raw_set(NEW, lua.create_function(|lua, ()| lua.create_table())?).unwrap();

    fmt::write(
        type_def,
        const_format_args!(
            "
---@class (exact) {TileEntity}Singleton
---@field {NEW} fun(): {TileEntity} Creates a {TileEntity} object.
local {TileEntity} = {{ }}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(TileEntity, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {TileEntity}
"
        ),
    )
    .unwrap();

    Ok(())
}
