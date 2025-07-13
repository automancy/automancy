pub mod game;
pub mod instances;

use crate::{model::ModelManager, renderer::instances::DrawInstanceManager};

#[derive(Debug, Default)]
pub struct AutomancyRenderState {
    pub model_man: ModelManager,
    pub instance_man: DrawInstanceManager,
}
