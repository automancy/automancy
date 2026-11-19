use automancy_data::{fake_static, lua::doc::LuaObjectDoc, rendering::colors::Rgba};
use automancy_macros::lua_class;

pub struct RgbaClass {}

#[lua_class]
impl RgbaClass {
    fn extra_doc() -> &'static str {
        fake_static!(format!("{}\n", Rgba::lua_object_doc()))
    }

    /// Creates a new `Rgba` object.
    fn new(_lua: &mlua::Lua) -> mlua::Result<Rgba> {
        Ok(Rgba::default())
    }
}
