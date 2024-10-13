use automancy_defs::{
    id::{Id, ModelId, RenderTagId},
    math::Matrix4,
};
use rhai::plugin::*;
use rhai::Module;
use rhai::{exported_module, Engine};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderCommand {
    Untrack {
        tag: RenderTagId,
        model: ModelId,
    },
    Track {
        tag: RenderTagId,
        model: ModelId,
    },
    Transform {
        tag: RenderTagId,
        model: ModelId,
        model_matrix: Matrix4,
    },
}

#[allow(non_snake_case)]
#[export_module]
mod render_stuff {
    pub fn Untrack(tag: Id, model: Id) -> RenderCommand {
        RenderCommand::Untrack {
            tag: RenderTagId(tag),
            model: ModelId(model),
        }
    }
    pub fn Track(tag: Id, model: Id) -> RenderCommand {
        RenderCommand::Track {
            tag: RenderTagId(tag),
            model: ModelId(model),
        }
    }
    pub fn Transform(tag: Id, model: Id, model_matrix: Matrix4) -> RenderCommand {
        RenderCommand::Transform {
            tag: RenderTagId(tag),
            model: ModelId(model),
            model_matrix,
        }
    }
}

pub(crate) fn register_render_stuff(engine: &mut Engine) {
    engine.register_static_module("Render", exported_module!(render_stuff).into());
}
