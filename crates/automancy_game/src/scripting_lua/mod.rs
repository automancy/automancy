#![allow(non_upper_case_globals)]

use core::cell::UnsafeCell;

mod module;
use automancy_data::game::coord::TileCoord;
pub use module::*;

pub mod globals {
    pub const CURRENT_NAMESPACE: &str = "CURRENT_NAMESPACE";
    pub const CURRENT_LOADING_SCRIPT_TYPE: &str = "CURRENT_LOADING_SCRIPT_TYPE";
    pub const CURRENT_LOADING_SCRIPT_ID: &str = "CURRENT_LOADING_SCRIPT_ID";
}

pub mod types {
    pub const Module: &str = "Module";
}

pub mod doc {
    use const_format::formatcp;

    use super::types::*;
    use crate::{actor::tile_entity::lua::types::TileEntity, scripting_lua::script::types::Script};

    #[rustfmt::skip]
    pub const MODULE: &str = formatcp!(
"---@class (exact) {Module}
---@field {TileEntity}? {TileEntity}
---@field {Script}? {Script}"
    );
}

pub(crate) fn create_readonly_metatable(lua: &mlua::Lua, table: mlua::Table) -> mlua::Result<mlua::Table> {
    let metatable = lua.create_table()?;

    thread_local! {
        static ERROR_MESSAGE: UnsafeCell<Option<mlua::LuaString>> = UnsafeCell::default();
        static NEW_INDEX_FUNCTION: UnsafeCell<Option<mlua::Function>> = UnsafeCell::default();
    }

    let new_index_function = NEW_INDEX_FUNCTION.with(|v| unsafe { &mut *v.get() });
    if new_index_function.is_none() {
        let error_message = ERROR_MESSAGE.with(|v| unsafe { &mut *v.get() });
        if error_message.is_none() {
            *error_message = Some(lua.create_string("attempt to update a read-only table")?);
        }

        let error_message = error_message.clone().unwrap();
        *new_index_function = Some(lua.create_function(move |lua: &mlua::Lua, _: mlua::Value| {
            lua.globals()
                .raw_get::<mlua::Function>("error")?
                .call::<()>((error_message.clone(), 2))?;
            Ok(())
        })?);
    }

    metatable.raw_set(mlua::MetaMethod::NewIndex.as_ref(), new_index_function.clone().unwrap())?;
    metatable.raw_set(
        mlua::MetaMethod::Index.as_ref(),
        lua.create_function(move |_: &mlua::Lua, _: mlua::Value| Ok(table.clone()))?,
    )?;

    Ok(metatable)
}

pub fn lua_log_err(called_func: &str, script_id: &str, err: &mlua::Error, coord: Option<TileCoord>) {
    let coord = coord.map(|v| v.to_minimal_string()).unwrap_or_else(|| "(no coord)".to_string());

    log::error!("At {coord}, In {script_id}, {called_func}: {err}");
}
