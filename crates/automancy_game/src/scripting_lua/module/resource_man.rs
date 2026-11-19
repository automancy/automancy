use core::fmt::{self};

use automancy_data::{
    gen_field_doc, gen_object_doc,
    lua::doc::{LuaField, LuaFieldDoc, LuaObjectDoc, LuaTypeName},
};
use mlua::prelude::IntoLua;

use crate::resources::{
    ResourceManager,
    registry::Registry,
    types::{category::CategoryDef, item::ItemDef, recipe::RecipeDef, tag::TagDef, tile::TileDef},
};

pub mod fields {
    pub const REGISTRY: &str = "registry";
}

impl LuaTypeName for ResourceManager {
    fn lua_name() -> &'static str {
        "ResourceMan"
    }
}

impl LuaFieldDoc<1> for ResourceManager {
    fn fields() -> [LuaField; 1] {
        [LuaField {
            name: fields::REGISTRY,
            ty: Registry::lua_name(),
            optional: false,
        }]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for ResourceManager {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

#[inline]
#[track_caller]
pub fn compile(resource_man: &ResourceManager, type_def: &mut String) -> mlua::Result<()> {
    use fields::*;

    let lua = &resource_man.lua;

    let resource_man_global = lua.create_table()?;

    {
        resource_man_global.raw_set(REGISTRY, resource_man.registry.into_lua(lua)?)?;
    }

    lua.globals().raw_set(ResourceManager::lua_name(), resource_man_global)?;

    fmt::write(
        type_def,
        const_format_args!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n{} = {{}}\n",
            TileDef::lua_object_doc(),
            ItemDef::lua_object_doc(),
            RecipeDef::lua_object_doc(),
            TagDef::lua_object_doc(),
            CategoryDef::lua_object_doc(),
            Registry::lua_object_doc(),
            ResourceManager::lua_object_doc(),
            ResourceManager::lua_name()
        ),
    )
    .unwrap();

    Ok(())
}
