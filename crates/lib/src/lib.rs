#![feature(const_option_ops)]
#![feature(const_trait_impl)]

pub static BUILD_PROFILE: &str = option_env!("BUILD_PROFILE").unwrap_or("dev");
pub static PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub fn version() -> String {
    format!("{}-{}", PKG_VERSION, BUILD_PROFILE)
}

pub mod integration;
//pub mod gui;
pub mod util;
