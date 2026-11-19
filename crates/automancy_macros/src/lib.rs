mod ids;
mod lua;

/**
Derives an Id registry with a convenient constructor that interns static strings as Ids.

Examples:
```
use automancy_macros::IdReg;
use automancy_data::id::Id;

#[derive(IdReg)]
pub struct FooIds {
    id_foo: Id,
    #[namespace("core")]
    id_bar: Id,
    #[namespace("meowzer")]
    #[name("zoo")]
    id_zoo: Id,
}
```

Invalid usages:
```compile_fail
#[derive(automancy_macros::IdReg)]
pub enum Foo {}

#[derive(automancy_macros::IdReg)]
pub struct Bar();
```
*/
#[proc_macro_derive(IdReg, attributes(name, namespace))]
pub fn derive_id_reg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    ids::derive_id_reg(input)
}

/**
 * Attribute macro only meant to be used internally in crate [`automancy_game`] for Lua interop.
 *
 * For how to use, please see [`automancy_game::scripting_lua::module`].
 */
#[proc_macro_attribute]
pub fn lua_class(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    lua::lua_class(attr, item)
}

/**
 * Attribute macro only meant to be used internally in crate [`automancy_game`] for Lua interop.
 *
 * For how to use, please see [`automancy_game::scripting_lua::module`].
 */
#[proc_macro_attribute]
pub fn lua_global_fn(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    lua::lua_global_fn(attr, item)
}
