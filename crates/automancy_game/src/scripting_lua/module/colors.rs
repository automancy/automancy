use core::fmt;

use automancy_data::rendering::colors::{
    Rgba,
    lua::{doc::RGBA, singleton::NEW, types::Rgba},
};

use crate::scripting_lua;

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    fmt::write(
        type_def,
        const_format_args!(
            "{RGBA}
"
        ),
    )
    .unwrap();

    let module = lua.create_table()?;

    module.raw_set(NEW, lua.create_function(|_lua, ()| Ok(Rgba::default()))?).unwrap();

    fmt::write(
        type_def,
        const_format_args!(
            "
---@class (exact) {Rgba}Singleton
---@field {NEW} fun(): {Rgba} Creates a {Rgba} object.
local {Rgba} = {{ }}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(Rgba, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {Rgba}
"
        ),
    )
    .unwrap();

    Ok(())
}
