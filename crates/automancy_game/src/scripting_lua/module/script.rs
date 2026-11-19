use automancy_data::{
    fake_static, gen_field_doc, gen_object_doc,
    lua::doc::{LuaField, LuaFieldDoc, LuaObjectDoc, LuaTypeName},
};
use automancy_macros::lua_class;

pub mod fields {
    pub const LOAD: &str = "load";
    pub const POST_LOAD: &str = "post_load";
    pub const UNIT_TEST: &str = "unit_test";
}

#[must_use]
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    derive_more::PartialEq,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
    derive_more::AsMut,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct Script(mlua::Table);

const impl LuaTypeName for Script {
    fn lua_name() -> &'static str {
        "Script"
    }
}

impl LuaFieldDoc<3> for Script {
    fn fields() -> [LuaField; 3] {
        [
            LuaField {
                name: fields::LOAD,
                ty: fake_static!(format!("fun(self: {}): nil", Self::lua_name())),
                optional: true,
            },
            LuaField {
                name: fields::POST_LOAD,
                ty: fake_static!(format!("fun(self: {}): nil", Self::lua_name())),
                optional: true,
            },
            LuaField {
                name: fields::UNIT_TEST,
                ty: fake_static!(format!("fun(self: {}): nil", Self::lua_name())),
                optional: true,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaObjectDoc for Script {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields)
    }
}

impl mlua::IntoLua for Script {
    fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Table(self.0))
    }
}

impl mlua::FromLua for Script {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let v = value.as_table().ok_or(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: stringify!($ty).to_string(),
            message: None,
        })?;

        Ok(Self(v.clone()))
    }
}

pub struct ScriptClass {}

#[lua_class]
impl ScriptClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!("{}\n", Script::lua_object_doc()))
    }

    /// Creates a `Script` object.
    fn new(lua: &mlua::Lua) -> mlua::Result<Script> {
        Ok(Script(lua.create_table()?))
    }
}
