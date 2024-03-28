use rhai::{Engine, Module};

use crate::types::function::{ResultType, TransactionResultType};

pub(crate) fn register_tile_stuff(engine: &mut Engine) {
    {
        let mut module = Module::new();

        module
            .set_var("MAKE_TRANSACTION", ResultType::MakeTransaction)
            .set_var("MAKE_EXTRACT_REQUEST", ResultType::MakeExtractRequest);

        engine.register_static_module("Result", module.into());
    }

    {
        let mut module = Module::new();

        module
            .set_var("PROXY", TransactionResultType::Proxy)
            .set_var("PASS_ON", TransactionResultType::PassOn)
            .set_var("CONSUME", TransactionResultType::Consume);

        engine.register_static_module("TransResult", module.into());
    }
}
