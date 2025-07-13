use automancy_data::{
    id::{Id, ModelId, RenderId},
    math::Matrix4,
};
use rhai::{Engine, Module, exported_module, plugin::*};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderCommand {
    Untrack {
        render_id: RenderId,
        model_id: ModelId,
    },
    Track {
        render_id: RenderId,
        model_id: ModelId,
    },
    Transform {
        render_id: RenderId,
        model_id: ModelId,
        model_matrix: Matrix4,
    },
}

#[allow(non_snake_case)]
#[export_module]
mod render_stuff {
    pub fn Untrack(render_id: Id, model_id: Id) -> RenderCommand {
        RenderCommand::Untrack {
            render_id: RenderId(render_id),
            model_id: ModelId(model_id),
        }
    }
    pub fn Track(render_id: Id, model_id: Id) -> RenderCommand {
        RenderCommand::Track {
            render_id: RenderId(render_id),
            model_id: ModelId(model_id),
        }
    }
    pub fn Transform(render_id: Id, model_id: Id, model_matrix: Matrix4) -> RenderCommand {
        RenderCommand::Transform {
            render_id: RenderId(render_id),
            model_id: ModelId(model_id),
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
                render_id: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
                model_id: ModelId(resource_man.registry.model_ids.tile_none),
            },
            RenderCommand::Transform {
                render_id: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
                model_id: ModelId(resource_man.registry.model_ids.tile_none),
                model_matrix: coord.as_translation(),
            },
        ]
    }

    pub fn untrack_none(resource_man: &ResourceManager) -> [RenderCommand; 1] {
        [RenderCommand::Untrack {
            render_id: RenderId(resource_man.registry.data_ids.none_tile_render_tag),
            model_id: ModelId(resource_man.registry.model_ids.tile_none),
        }]
    }
}

pub(crate) fn register_render_stuff(engine: &mut Engine) {
    engine.register_static_module("Render", exported_module!(render_stuff).into());
}
