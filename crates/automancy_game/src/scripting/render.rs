use automancy_data::{
    id::{Id, ModelId, RenderId},
    math::Matrix4,
};
use rhai::{Engine, Module, exported_module, plugin::*};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderCommand {
    Clear,
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

pub mod util {
    use automancy_data::{
        game::coord::TileCoord,
        id::{ModelId, RenderId},
    };

    use crate::{resources::ResourceManager, scripting::render::RenderCommand};

    pub fn track_none(resource_man: &ResourceManager, coord: TileCoord) -> [RenderCommand; 2] {
        [
            RenderCommand::Track {
                tag: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
                model: ModelId(resource_man.registry.model_ids.tile_none),
            },
            RenderCommand::Transform {
                tag: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
                model: ModelId(resource_man.registry.model_ids.tile_none),
                model_matrix: coord.as_translation(),
            },
        ]
    }

    pub fn untrack_none(resource_man: &ResourceManager) -> [RenderCommand; 1] {
        [RenderCommand::Untrack {
            tag: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
            model: ModelId(resource_man.registry.model_ids.tile_none),
        }]
    }
}

pub(crate) fn register_render_stuff(engine: &mut Engine) {
    engine.register_static_module("Render", exported_module!(render_stuff).into());
}
