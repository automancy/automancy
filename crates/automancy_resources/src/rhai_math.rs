use automancy_defs::math::Matrix4;
use rhai::{Engine, Module};
use std::ops::Mul;

pub(crate) fn register_math_stuff(engine: &mut Engine) {
    let mut module = Module::new();

    module.set_var("IDENTITY", Matrix4::IDENTITY);

    engine.register_static_module("Matrix", module.into());

    engine
        .register_type_with_name::<Matrix4>("Matrix")
        .register_fn("*", <Matrix4 as Mul>::mul);
}
