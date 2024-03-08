pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub static LOGO_PATH: &str = "assets/logo.png";
pub static LOGO: &[u8] = include_bytes!("assets/logo.png");
pub static SSAO_NOISE_MAP: &[u8] = include_bytes!("assets/noise_map.png");

pub mod camera;
pub mod event;
pub mod game;
pub mod gpu;
pub mod gui;
pub mod input;
pub mod map;
pub mod options;
pub mod renderer;
pub mod setup;
pub mod tile_entity;
pub mod util;
