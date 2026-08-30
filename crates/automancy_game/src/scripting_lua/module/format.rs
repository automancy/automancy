use core::fmt::{self, Display};
use std::collections::HashMap;

use automancy_data::{
    game::{
        coord::{TileCoord, lua::types::TileCoord},
        generic::{DataMap, lua::types::DataMap},
    },
    id::{Id, lua::types::Id},
};

use crate::{
    scripting_lua,
    scripting_lua::format::{singleton::*, types::*},
};

#[derive(Debug, Clone)]
pub enum LuaFormattable {
    Id(Id),
    DataMap(Box<DataMap>),
    TileCoord(TileCoord),
    String(mlua::LuaString),
}

const LOG: &str = "log";

pub mod types {
    pub const Format: &str = "Format";
}

pub mod singleton {
    pub const ID: &str = "id";
    pub const DATA: &str = "data";
    pub const COORD: &str = "coord";
    pub const STRING: &str = "string";
}

impl Display for LuaFormattable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LuaFormattable::Id(v) => v.fmt(f),
            LuaFormattable::DataMap(v) => v.fmt(f),
            LuaFormattable::TileCoord(v) => v.fmt(f),
            LuaFormattable::String(v) => v.display().fmt(f),
        }
    }
}

impl mlua::UserData for LuaFormattable {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, this, ()| Ok(this.to_string()));
    }
}

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    fmt::write(
        type_def,
        const_format_args!(
            "---@alias {Format} userdata
"
        ),
    )
    .unwrap();

    {
        lua.globals().raw_set(
            "print",
            lua.create_function(|_lua, args: mlua::MultiValue| {
                log::info!("{}", args.into_iter().map(|v| format!("{:#?}", v)).collect::<Vec<_>>().join("\t"));

                Ok(())
            })?,
        )?;

        lua.globals().raw_set(
            LOG,
            lua.create_function(|_lua, (format, context): (mlua::BorrowedStr, mlua::Table)| {
                #[allow(clippy::mutable_key_type)]
                let mut owned = HashMap::new();

                for pair in context.pairs::<mlua::BorrowedStr, mlua::AnyUserData>() {
                    let (name, value) = pair?;
                    owned.insert(name, value.borrow::<LuaFormattable>()?);
                }

                let mut borrowed = HashMap::new();
                for (k, v) in &owned {
                    borrowed.insert(k.as_ref(), interpolator::Formattable::display(v));
                }

                log::info!("{LOG}: {}", interpolator::format(&format, &borrowed).unwrap());

                Ok(())
            })?,
        )?;
        fmt::write(
            type_def,
            const_format_args!(
                "
---Logs with the provided format and a table of names and `Format` values.
---
---Example:
---```
---{LOG}(\"this is my coord: {{coord}}, and my id: {{id}}\", {{
---\tcoord = Format.coord(TileCoord.new(1, 2)),
---\tid = Format.id(Id.Id(\"foo\"))
---}})
---
---=> Prints: `this is my coord: [1, 2], and my id: Id(automancy:foo)`
---```
---@param format string
---@param context table<string, Format>
function {LOG}(format, context) end
"
            ),
        )
        .unwrap();
    }

    let module = lua.create_table()?;

    module.raw_set(ID, lua.create_function(|_lua, v: Id| Ok(LuaFormattable::Id(v)))?)?;
    module.raw_set(
        DATA,
        lua.create_function(|_lua, v: DataMap| Ok(LuaFormattable::DataMap(Box::new(v))))?,
    )?;
    module.raw_set(COORD, lua.create_function(|_lua, v: TileCoord| Ok(LuaFormattable::TileCoord(v)))?)?;
    module.raw_set(
        STRING,
        lua.create_function(|_lua, v: mlua::LuaString| Ok(LuaFormattable::String(v)))?,
    )?;

    fmt::write(
        type_def,
        const_format_args!(
            "
---@see {LOG}
---@class (exact) {Format}Singleton
---@field {ID} fun(v: {Id}): {Format} Turns the {Id} into a {Format} object to be used in `{LOG}`.
---@field {DATA} fun(v: {DataMap}): {Format} Turns the {DataMap} into a {Format} object to be used in `{LOG}`.
---@field {COORD} fun(v: {TileCoord}): {Format} Turns the {TileCoord} into a {Format} object to be used in `{LOG}`.
---@field {STRING} fun(v: string): {Format} Turns the string into a {Format} object to be used in `{LOG}`.
local {Format} = {{}}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(Format, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {Format}
"
        ),
    )
    .unwrap();

    Ok(())
}
