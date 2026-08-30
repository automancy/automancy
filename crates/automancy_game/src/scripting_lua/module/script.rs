use core::fmt;

use crate::{scripting_lua, scripting_lua::script::doc::SCRIPT};

pub mod types {
    pub const Script: &str = "Script";
}

pub mod fields {
    pub const LOAD: &str = "load";
    pub const UNIT_TEST: &str = "unit_test";
}

pub mod singleton {
    pub const NEW: &str = "new";
}

pub mod doc {
    use const_format::formatcp;

    use super::{fields::*, types::*};

    #[rustfmt::skip]
    pub const SCRIPT: &str = formatcp!(
"---@class (exact) {Script}
---@field {LOAD}? fun(self: {Script}): nil
---@field {UNIT_TEST}? fun(self: {Script}): nil"
    );
}

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    use singleton::*;
    use types::*;

    fmt::write(
        type_def,
        const_format_args!(
            "{SCRIPT}
"
        ),
    )
    .unwrap();

    let module = lua.create_table()?;

    module.raw_set(NEW, lua.create_function(|lua, ()| lua.create_table())?).unwrap();

    fmt::write(
        type_def,
        const_format_args!(
            "
---@class (exact) {Script}Singleton
---@field {NEW} fun(): {Script} Creates a {Script} object.
local {Script} = {{ }}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(Script, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {Script}
"
        ),
    )
    .unwrap();

    Ok(())
}
