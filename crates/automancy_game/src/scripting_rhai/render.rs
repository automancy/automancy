use automancy_data::{
    id::{Id, ModelId, RenderId},
    math::Matrix4,
};
use rhai::{Engine, exported_module, plugin::*};

#[allow(non_snake_case)]
#[export_module]
mod render_stuff {
    use crate::script::RenderCommand;

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
    pub fn Untrack(render_id: Id, model_id: Id) -> RenderCommand {
        RenderCommand::Untrack {
            render_id: RenderId(render_id),
            model_id: ModelId(model_id),
        }
    }
}

pub mod util {
    use automancy_data::game::coord::TileCoord;

    use crate::{resources::ResourceManager, script::RenderCommand};

    pub fn track_none(resource_man: &ResourceManager, coord: TileCoord) -> [RenderCommand; 2] {
        [
            RenderCommand::Track {
                render_id: resource_man.registry.render_ids.empty_tiles,
                model_id: resource_man.registry.model_ids.tile_none,
            },
            RenderCommand::Transform {
                render_id: resource_man.registry.render_ids.empty_tiles,
                model_id: resource_man.registry.model_ids.tile_none,
                model_matrix: coord.as_translation(),
            },
        ]
    }

    pub fn untrack_none(resource_man: &ResourceManager) -> [RenderCommand; 1] {
        [RenderCommand::Untrack {
            render_id: resource_man.registry.render_ids.empty_tiles,
            model_id: resource_man.registry.model_ids.tile_none,
        }]
    }
}

pub(crate) fn register_render_stuff(engine: &mut Engine) {
    engine.register_static_module("Render", exported_module!(render_stuff).into());
}
