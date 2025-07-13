use std::time::Instant;

use automancy_game::resources::ResourceManager;

use crate::{
    gpu::RenderResources,
    renderer::{game_render::GameRenderer, instance::DrawInstanceManager, model::ModelManager},
};

pub mod game_render;
pub mod instance;
pub mod model;

#[derive(Debug, Default)]
pub struct AutomancyRenderState {
    pub model_man: ModelManager,
    pub instance_man: DrawInstanceManager,
}

pub struct AutomancyRendering {
    pub renderer: GameRenderer,
    pub res: RenderResources,

    /// the last frame's starting time
    pub frame_start: Instant,

    pub screenshotting: bool,
}

impl AutomancyRendering {
    pub async fn new(resource_man: &ResourceManager, render_state: &AutomancyRenderState, window: impl Into<wgpu::SurfaceTarget<'static>>) -> Self {
        let res = RenderResources::new(resource_man, window, &render_state.model_man.vertices, &render_state.model_man.indices).await;

        Self {
            renderer: GameRenderer::new(),
            res,

            frame_start: Instant::now(),

            screenshotting: false,
        }
    }
}
