use automancy_data::game::coord::TileCoord;
use rhai::{CallFnOptions, Dynamic};

pub mod coord;
pub mod data;
pub mod math;
pub mod render;
pub mod tile;
pub mod ui;
pub mod util;

pub fn rhai_call_options<'a>(state: &'a mut Dynamic) -> CallFnOptions<'a> {
    CallFnOptions::new().eval_ast(false).rewind_scope(true).bind_this_ptr(state)
}

pub fn rhai_log_err(called_func: &str, script_id: &str, err: &rhai::EvalAltResult, coord: Option<TileCoord>) {
    let coord = coord.map(|v| v.to_minimal_string()).unwrap_or_else(|| "(no coord)".to_string());

    match err {
        rhai::EvalAltResult::ErrorFunctionNotFound(name, ..) => {
            if name != called_func {
                log::error!("At {coord}, In {script_id}, {called_func}: {err}");
            }
        },
        _ => {
            log::error!("At {coord}, In {script_id}, {called_func}: {err}");
        },
    }
}
