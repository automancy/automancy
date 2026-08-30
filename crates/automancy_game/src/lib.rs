#![feature(trim_prefix_suffix)]
#![feature(iterator_try_collect)]
#![feature(const_format_args)]
#![feature(const_cmp)]
#![feature(const_convert)]
#![feature(const_trait_impl)]
#![feature(const_option_ops)]

pub mod pkg {
    pub static BUILD_PROFILE: &str = option_env!("BUILD_PROFILE").unwrap_or("dev");
    pub static PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn version() -> String {
        format!("{}-{}", PKG_VERSION, BUILD_PROFILE)
    }
}

pub mod actor;
pub mod format;
pub mod input;
pub mod persistent;
pub mod resources;
pub mod script;
pub mod scripting_lua;
pub mod scripting_rhai;
pub mod state;

#[cfg(miri)]
pub mod miri;
