use std::sync::Arc;

use automancy_game::resources::ResourceManager;
use automancy_rendering::{
    gpu::RenderResources,
    renderer::{AutomancyRenderState, game::GameRenderer},
};
use winit::window::Window;

pub struct AutomancyRendering {
    pub renderer: GameRenderer,
    pub res: RenderResources,

    pub screenshotting: bool,
}

impl AutomancyRendering {
    pub async fn new(
        resource_man: &ResourceManager,
        render_state: &AutomancyRenderState,
        window: Arc<Window>,
    ) -> Self {
        let res = RenderResources::new(
            resource_man,
            window,
            &render_state.model_man.vertices,
            &render_state.model_man.indices,
        )
        .await;

        Self {
            renderer: GameRenderer::new(),
            res,
            screenshotting: false,
        }
    }
}
