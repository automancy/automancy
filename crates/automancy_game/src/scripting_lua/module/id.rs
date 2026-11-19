use automancy_data::{
    all_ids, fake_static, for_each_id,
    id::*,
    lua::doc::{LuaAliasDoc, LuaObjectDoc},
};
use automancy_macros::lua_class;

use crate::scripting_lua;

pub struct IdClass {}

macro_rules! impl_id {
    ($ty:ident) => {
        paste::paste! {
            fn [<new_ $ty:snake>](lua: &mlua::Lua, id: mlua::BorrowedStr) -> mlua::Result<$ty> {
                let script_context = lua
                    .globals()
                    .raw_get::<mlua::BorrowedStr>(scripting_lua::globals::CURRENT_LOADING_SCRIPT_TYPE)?;
                let script_id = lua
                    .globals()
                    .raw_get::<mlua::BorrowedStr>(scripting_lua::globals::CURRENT_LOADING_SCRIPT_ID)?;
                let namespace = lua
                    .globals()
                    .raw_get::<mlua::BorrowedStr>(scripting_lua::globals::CURRENT_NAMESPACE)
                    .ok();

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

                Ok(id)
            }
        }
    };
}

impl IdClass {
    for_each_id! {
        impl_id!(Id);
    }
}

macro_rules! impl_id_class {
    ($($ty:ident,)*) => {
        paste::paste! {
            #[lua_class]
            impl IdClass {
                fn extra_doc() -> &'static str {
                    let mut string = String::new();
                    $(
                        writeln!(&mut string, "{}", $ty::lua_object_doc()).unwrap();
                    )*

                    writeln!(&mut string, "{}", InnerId::lua_alias_doc()).unwrap();

                    fake_static!(string)
                }

                $(
                    #[allow(non_snake_case)]
                    #[doc = concat!("Creates an interned ", stringify!($ty), " from a string.")]
                    fn $ty(lua: &mlua::Lua, string: mlua::BorrowedStr) -> mlua::Result<$ty> {
                        Self::[<new_ $ty:snake>](lua, string)
                    }
                )*

                $(
                    #[allow(non_snake_case)]
                    #[doc = concat!("Casts another Id into a(n) ", stringify!($ty))]
                    fn [<As $ty>](_: &mlua::Lua, id: InnerId) -> mlua::Result<$ty> {
                        Ok($ty::from(Id::from(id)))
                    }
                )*
            }
        }
    };
}

all_ids! {
    impl_id_class!(..);
}
