use core::fmt;

use automancy_data::{
    for_each_id_new_type,
    id::{lua::types::Id, *},
};
use const_format::formatcp;

use crate::scripting_lua::{
    globals::{CURRENT_LOADING_SCRIPT_ID, CURRENT_LOADING_SCRIPT_TYPE, CURRENT_NAMESPACE},
    {self},
};

trait LuaId {
    const DOC: &str;

    fn register_lua(lua: &mlua::Lua, module: &mlua::Table, type_def: &mut String) -> mlua::Result<()>;
}

macro_rules! impl_id {
    ($ty:ident) => {
        impl LuaId for $ty {
            const DOC: &str = formatcp!(
                "---@alias {} {}",
                stringify!($ty),
                match stringify!($ty) {
                    "Id" => "integer",
                    _ => "Id",
                }
            );

            fn register_lua(lua: &mlua::Lua, module: &mlua::Table, type_def: &mut String) -> mlua::Result<()> {
                const TYPE: &str = stringify!($ty);

                module.raw_set(
                    stringify!($ty),
                    lua.create_function_mut(|lua, id: mlua::BorrowedStr| {
                        let script_context = lua.globals().raw_get::<mlua::BorrowedStr>(CURRENT_LOADING_SCRIPT_TYPE)?;
                        let script_id = lua.globals().raw_get::<mlua::BorrowedStr>(CURRENT_LOADING_SCRIPT_ID)?;

                        let namespace = lua.globals().raw_get::<mlua::BorrowedStr>(CURRENT_NAMESPACE).ok();

                        let id = MUTABLE_GLOBAL_INTERNER.with_borrow_mut(|interner| {
                            $ty::from(
                                interner
                                    .as_mut()
                                    .expect("interning Ids is only possible while ResourceManager is loading resources.")
                                    .get_or_intern(&id, namespace.as_deref())
                                    .unwrap(),
                            )
                        });
                        log::info!("Interning {id} into scope of {script_context} script '{script_id}'.");

                        Ok(id.into_inner())
                    })?,
                )?;

                fmt::write(
                    type_def,
                    const_format_args!(
                        "
---@field {TYPE} fun(id: string): {TYPE} Creates an interned {TYPE} from a string."
                    ),
                )
                .unwrap();

                Ok(())
            }
        }
    };
}

impl_id!(Id);

for_each_id_new_type! {
    impl_id!(NewType);
}

#[inline]
#[track_caller]
pub(super) fn register_lua(lua: &mlua::Lua, type_def: &mut String) -> mlua::Result<()> {
    macro_rules! write_alias {
        ($ty:ty) => {
            fmt::write(type_def, const_format_args!("{}\n", <$ty>::DOC)).unwrap();
        };
    }
    write_alias!(Id);
    for_each_id_new_type! {
        write_alias!(NewType);
    };

    let module = lua.create_table()?;

    fmt::write(
        type_def,
        const_format_args!(
            "
---@class (exact) {Id}Singleton"
        ),
    )
    .unwrap();
    Id::register_lua(lua, &module, type_def)?;
    for_each_id_new_type! {
        NewType::register_lua(lua, &module, type_def)?;
    };
    fmt::write(
        type_def,
        const_format_args!(
            "
local {Id} = {{}}
"
        ),
    )
    .unwrap();

    module.set_metatable(Some(scripting_lua::create_readonly_metatable(lua, module.clone())?))?;
    lua.register_module(Id, module)?;
    fmt::write(
        type_def,
        const_format_args!(
            "
return {Id}
"
        ),
    )
    .unwrap();

    Ok(())
}
