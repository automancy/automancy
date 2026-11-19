use automancy_data::{
    fake_static,
    game::{coord::TileCoord, generic::DataMap},
    gen_field_doc, gen_method_doc, gen_object_doc,
    id::TileId,
    lua::doc::{LuaArg, LuaField, LuaFieldDoc, LuaMethod, LuaMethodDoc, LuaObjectDoc, LuaTypeName},
};
use automancy_macros::lua_class;
use const_format::formatcp;

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
pub struct TileEntity(pub mlua::Table);

pub mod fields {
    pub const DATA: &str = "data";
    pub const ID: &str = "id";
    pub const COORD: &str = "coord";
    pub const MESSAGES: &str = "messages";
    pub const LOAD: &str = "load";
    pub const HANDLE_TICK: &str = "handle_tick";

    pub const MSG_COORD: &str = "coord";
    pub const MSG_MSG: &str = "msg";
}

const impl LuaTypeName for TileEntity {
    fn lua_name() -> &'static str {
        "TileEntity"
    }
}

impl LuaFieldDoc<4> for TileEntity {
    fn fields() -> [LuaField; 4] {
        use fields::{MSG_COORD, MSG_MSG};

        [
            LuaField {
                name: fields::DATA,
                ty: DataMap::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::ID,
                ty: TileId::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::COORD,
                ty: TileCoord::lua_name(),
                optional: false,
            },
            LuaField {
                name: fields::MESSAGES,
                ty: formatcp!("{{ {MSG_COORD}: TileCoord, {MSG_MSG}: TileMsg }}[]"), // TODO replace
                optional: false,
            },
        ]
    }

    fn lua_field_doc() -> &'static str {
        gen_field_doc!()
    }
}

impl LuaMethodDoc<1, 2> for TileEntity {
    fn methods() -> [LuaMethod<1>; 2] {
        [
            LuaMethod {
                name: fields::LOAD,
                args: [LuaArg {
                    name: "self",
                    ty: TileEntity::lua_name(),
                }],
                return_ty: "",
            },
            LuaMethod {
                name: fields::HANDLE_TICK,
                args: [LuaArg {
                    name: "self",
                    ty: TileEntity::lua_name(),
                }],
                return_ty: "",
            },
        ]
    }

    fn lua_method_doc() -> &'static str {
        gen_method_doc!()
    }
}

impl LuaObjectDoc for TileEntity {
    fn lua_object_doc() -> &'static str {
        gen_object_doc!(fields, methods)
    }
}

impl mlua::IntoLua for TileEntity {
    fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Table(self.0))
    }
}

impl mlua::FromLua for TileEntity {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let v = value.as_table().ok_or(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: stringify!($ty).to_string(),
            message: None,
        })?;

        Ok(Self(v.clone()))
    }
}

pub struct TileEntityClass {}

#[lua_class]
impl TileEntityClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!("{}\n", TileEntity::lua_object_doc()))
    }

    /// Creates a new `TileEntity` object.
    fn new(lua: &mlua::Lua) -> mlua::Result<TileEntity> {
        Ok(TileEntity(lua.create_table()?))
    }
}
