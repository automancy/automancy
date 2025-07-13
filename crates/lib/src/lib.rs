pub static PROFILE: &str = option_env!("BUILD_PROFILE").unwrap_or("dev");
pub static VERSION: &str = env!("CARGO_PKG_VERSION") + "-" + PROFILE;

pub use anyhow;
pub use automancy_defs::*;
pub use automancy_macros::*;
pub use automancy_resources::*;
pub use automancy_system::*;
pub use automancy_ui::*;
pub use bytemuck;
pub use cosmic_text;
pub use hashbrown;
pub use log;
pub use ractor;
pub use rhai;
pub use ron;
pub use serde;
pub use thiserror;
pub use tokio;
pub use uuid;
pub use walkdir;
pub use wgpu;
pub use winit;
pub use yakui;
pub use yakui_wgpu;
pub use yakui_winit;

pub mod event;
pub mod gpu;
pub mod gui;
pub mod renderer;
pub mod ui_game_object;
pub mod util;

pub type GameState = InnerGameState<renderer::GameRenderer>;
