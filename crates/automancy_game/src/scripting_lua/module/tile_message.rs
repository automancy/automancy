use automancy_data::{fake_static, lua::doc::LuaObjectDoc};
use automancy_macros::lua_class;

use crate::actor::message::{TileMsg, TileTransaction};

pub struct TileMsgClass {}

#[lua_class]
impl TileMsgClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!("{}\n\n{}\n", TileMsg::lua_object_doc(), TileTransaction::lua_object_doc()))
    }

    /// Creates a TileCoord from a `q` component and a `r` component.
    fn transaction(_lua: &mlua::Lua, v: TileTransaction) -> mlua::Result<TileMsg> {
        Ok(TileMsg::Transaction(v))
    }
}
