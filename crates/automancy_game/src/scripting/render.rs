use automancy_data::{
    id::{Id, ModelId, RenderId},
    math::Matrix4,
};
use rhai::{Engine, Module, exported_module, plugin::*};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderCommand {
    Untrack {
        tag: RenderId,
        model: ModelId,
    },
    Track {
        tag: RenderId,
        model: ModelId,
    },
    Transform {
        tag: RenderId,
        model: ModelId,
        model_matrix: Matrix4,
    },
}

#[allow(non_snake_case)]
#[export_module]
mod render_stuff {
    pub fn Untrack(tag: Id, model: Id) -> RenderCommand {
        RenderCommand::Untrack {
            tag: RenderId(tag),
            model: ModelId(model),
        }
    }
    pub fn Track(tag: Id, model: Id) -> RenderCommand {
        RenderCommand::Track {
            tag: RenderId(tag),
            model: ModelId(model),
        }
    }
    pub fn Transform(tag: Id, model: Id, model_matrix: Matrix4) -> RenderCommand {
        RenderCommand::Transform {
            tag: RenderId(tag),
            model: ModelId(model),
            model_matrix,
        }
    }
}

pub(crate) fn register_render_stuff(engine: &mut Engine) {
    engine.register_static_module("Render", exported_module!(render_stuff).into());
}
