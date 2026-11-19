use automancy_data::{
    game::{generic::DataMap, inventory::ItemStack},
    gen_field_doc, gen_object_doc,
    id::*,
    id_map::{ImmutableIdMap, ImmutableIdSet},
    lua::doc::{LuaField, LuaFieldDoc, LuaObjectDoc, LuaTypeName},
    math::Int,
    rendering::GenericModel,
};

use crate::resources::{
    registry::Registry,
    types::{category::CategoryDef, item::ItemDef, recipe::RecipeDef, tag::TagDef, tile::TileDef},
};

mod fields {
    pub const TILE_DEFS: &str = "tile_defs";
    pub const ITEM_DEFS: &str = "item_defs";
    pub const RECIPE_DEFS: &str = "recipe_defs";
    pub const TAG_DEFS: &str = "tag_defs";
    pub const CATEGORY_DEFS: &str = "category_defs";
}

impl LuaTypeName for Registry {
    fn lua_name() -> &'static str {
        "Registry"
    }
}

impl LuaFieldDoc<5> for Registry {
    fn fields() -> [LuaField; 5] {
        [
            LuaField {
                name: fields::TILE_DEFS,
                ty: ImmutableIdMap::<TileId, TileDef>::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::ITEM_DEFS,
                ty: ImmutableIdMap::<ItemId, ItemDef>::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::RECIPE_DEFS,
                ty: ImmutableIdMap::<RecipeId, RecipeDef>::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::TAG_DEFS,
                ty: ImmutableIdMap::<TagId, TagDef>::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::CATEGORY_DEFS,
                ty: ImmutableIdMap::<CategoryId, CategoryDef>::lua_name(),
                optional: false,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for Registry {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for &Registry {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;

        table.raw_set(
            fields::TILE_DEFS,
            lua.create_table_from(self.tile_defs.iter().flat_map(|(k, v)| v.into_lua(lua).map(|v| (k, v))))?,
        )?;
        table.raw_set(
            fields::ITEM_DEFS,
            lua.create_table_from(self.item_defs.iter().flat_map(|(k, v)| v.into_lua(lua).map(|v| (k, v))))?,
        )?;
        table.raw_set(
            fields::RECIPE_DEFS,
            lua.create_table_from(self.recipe_defs.iter().flat_map(|(k, v)| v.into_lua(lua).map(|v| (k, v))))?,
        )?;
        table.raw_set(
            fields::TAG_DEFS,
            lua.create_table_from(self.tag_defs.iter().flat_map(|(k, v)| v.into_lua(lua).map(|v| (k, v))))?,
        )?;
        /* table.raw_set(
            fields::CATEGORY_DEFS,
            lua.create_table_from(self.category_defs.iter().flat_map(|(k, v)| v.into_lua(lua).map(|v| (k, v))))?,
        )?; */

        Ok(mlua::Value::Table(table))
    }
}

mod tile_def {
    pub mod fields {
        pub const SCRIPT: &str = "script";
        pub const CATEGORY: &str = "category";
        pub const DATA: &str = "data";
    }
}

mod item_def {
    pub mod fields {
        pub const MODEL: &str = "model";
    }
}

mod recipe_def {
    pub mod fields {
        pub const INPUTS: &str = "inputs";
        pub const OUTPUTS: &str = "outputs";
    }
}

mod tag_def {
    pub mod fields {
        pub const ENTRIES: &str = "entries";
    }
}

mod category_def {
    pub mod fields {
        pub const ORD: &str = "ord";
        pub const ICON: &str = "icon";
        pub const ITEM: &str = "item";
    }
}

impl LuaTypeName for TileDef {
    fn lua_name() -> &'static str {
        "TileDef"
    }
}

impl LuaFieldDoc<3> for TileDef {
    fn fields() -> [LuaField; 3] {
        use tile_def::fields;

        [
            LuaField {
                name: fields::SCRIPT,
                ty: ScriptId::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::CATEGORY,
                ty: CategoryId::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::DATA,
                ty: DataMap::lua_name(),
                optional: false,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for TileDef {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for &TileDef {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        use tile_def::fields;

        let table = lua.create_table()?;
        table.raw_set(fields::SCRIPT, self.script)?;
        table.raw_set(fields::CATEGORY, self.category)?;
        table.raw_set(fields::DATA, self.data.clone())?;

        Ok(mlua::Value::Table(table))
    }
}

impl LuaTypeName for ItemDef {
    fn lua_name() -> &'static str {
        "ItemDef"
    }
}

impl LuaFieldDoc<1> for ItemDef {
    fn fields() -> [LuaField; 1] {
        use item_def::fields;

        [LuaField {
            name: fields::MODEL,
            ty: ModelId::lua_name(),
            optional: false,
        }]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for ItemDef {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for &ItemDef {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        use item_def::fields;

        let table = lua.create_table()?;
        table.raw_set(fields::MODEL, self.model)?;

        Ok(mlua::Value::Table(table))
    }
}

impl LuaTypeName for RecipeDef {
    fn lua_name() -> &'static str {
        "RecipeDef"
    }
}

impl LuaFieldDoc<2> for RecipeDef {
    fn fields() -> [LuaField; 2] {
        use recipe_def::fields;

        [
            LuaField {
                name: fields::INPUTS,
                ty: Vec::<ItemStack>::lua_name(),
                optional: true,
            },
            LuaField {
                name: fields::OUTPUTS,
                ty: Vec::<ItemStack>::lua_name(),
                optional: false,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for RecipeDef {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for &RecipeDef {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        use recipe_def::fields;

        let table = lua.create_table()?;
        table.raw_set(fields::INPUTS, self.inputs.clone())?;
        table.raw_set(fields::OUTPUTS, self.outputs.clone())?;

        Ok(mlua::Value::Table(table))
    }
}

impl LuaTypeName for TagDef {
    fn lua_name() -> &'static str {
        "TagDef"
    }
}

impl LuaFieldDoc<1> for TagDef {
    fn fields() -> [LuaField; 1] {
        use tag_def::fields;

        [LuaField {
            name: fields::ENTRIES,
            ty: ImmutableIdSet::<Id>::lua_name(),
            optional: true,
        }]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for TagDef {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for &TagDef {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        use tag_def::fields;

        let table = lua.create_table()?;
        table.raw_set(fields::ENTRIES, &self.entries)?;

        Ok(mlua::Value::Table(table))
    }
}

impl LuaTypeName for CategoryDef {
    fn lua_name() -> &'static str {
        "CategoryDef"
    }
}

impl LuaFieldDoc<3> for CategoryDef {
    fn fields() -> [LuaField; 3] {
        use category_def::fields;

        [
            LuaField {
                name: fields::ORD,
                ty: Int::lua_name(),
                optional: true,
            },
            LuaField {
                name: fields::ICON,
                ty: GenericModel::lua_name(),
                optional: true,
            },
            LuaField {
                name: fields::ITEM,
                ty: ItemId::lua_name(),
                optional: true,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for CategoryDef {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}
