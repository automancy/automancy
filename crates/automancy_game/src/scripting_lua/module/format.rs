use core::fmt::Display;
use std::collections::HashMap;

use automancy_data::{
    fake_static,
    game::{coord::TileCoord, generic::DataMap},
    gen_object_doc,
    id::{Id, InnerId},
    lua::{
        TypedTable,
        doc::{LuaObjectDoc, LuaTypeName},
    },
};
use automancy_macros::lua_class;

#[derive(Debug, Clone, mlua::UserData, mlua::FromLua)]
pub enum LuaFormattable {
    Id(Id),
    DataMap(Box<DataMap>),
    TileCoord(TileCoord),
    String(mlua::LuaString),
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

#[mlua::userdata_impl]
impl LuaFormattable {
    #[lua(meta, infallible)]
    fn __tostring(&self) -> String {
        self.to_string()
    }
}

const impl LuaTypeName for LuaFormattable {
    fn lua_name() -> &'static str {
        "Formattable"
    }
}

impl LuaObjectDoc for LuaFormattable {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!()
    }
}

pub struct FormatClass {}

#[lua_class]
impl FormatClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!("{}\n", LuaFormattable::lua_object_doc()))
    }

    #[allow(clippy::mutable_key_type)]
    fn log(_lua: &mlua::Lua, format: mlua::BorrowedStr, context: TypedTable<mlua::BorrowedStr, LuaFormattable>) -> mlua::Result<()> {
        let mut owned = HashMap::new();

        for pair in context.pairs() {
            let (name, value) = pair?;
            owned.insert(name, value);
        }

        let mut borrowed = HashMap::new();
        for (k, v) in &owned {
            borrowed.insert(k.as_ref(), interpolator::Formattable::display(v));
        }

        log::info!("log: {}", interpolator::format(&format, &borrowed).unwrap());

        Ok(())
    }

    /// Turns the Id into a Format object to be used in `log`.
    fn id(_lua: &mlua::Lua, v: InnerId) -> mlua::Result<LuaFormattable> {
        Ok(LuaFormattable::Id(Id::from(v)))
    }

    /// Turns the DataMap into a Format object to be used in `log`.
    fn data(_lua: &mlua::Lua, v: DataMap) -> mlua::Result<LuaFormattable> {
        Ok(LuaFormattable::DataMap(Box::new(v)))
    }

    /// Turns the TileCoord into a Format object to be used in `log`.
    fn coord(_lua: &mlua::Lua, v: TileCoord) -> mlua::Result<LuaFormattable> {
        Ok(LuaFormattable::TileCoord(v))
    }

    /// Turns the string into a Format object to be used in `log`.
    fn string(_lua: &mlua::Lua, v: mlua::LuaString) -> mlua::Result<LuaFormattable> {
        Ok(LuaFormattable::String(v))
    }
}
