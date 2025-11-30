use core::time::Duration;
use std::time::Instant;

use automancy_game::resources::ResourceManager;

mod game_render;
mod render_state;

pub use game_render::*;
pub use render_state::*;

pub struct AutomancyRendering {
    pub game_renderer: GameRenderer,
    pub res: crate::gpu::RenderResources,

    /// the last frame's starting time
    pub frame_start: Instant,
    /// the last frame's duration
    pub frame_time: Duration,
    pub frame_count: u32,

    pub screenshotting: bool,
}

impl AutomancyRendering {
    pub async fn new(
        resource_man: &ResourceManager,
        render_state: &AutomancyRenderState,
        window_handle: impl Into<wgpu::SurfaceTarget<'static>>,
        display_handle: impl wgpu::wgt::WgpuHasDisplayHandle,
    ) -> Self {
        let res = crate::gpu::RenderResources::new(resource_man, render_state, window_handle, display_handle).await;

        Self {
            game_renderer: GameRenderer::new(),
            res,

            frame_start: Instant::now(),
            frame_time: Duration::ZERO,
            frame_count: 0,

            screenshotting: false,
        }
    }
}
