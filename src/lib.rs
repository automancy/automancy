#![feature(slice_group_by)]
#![feature(duration_consts_float)]
#![feature(result_option_inspect)]

pub static IOSEVKA_FONT: &[u8] = include_bytes!("../compile/fonts/iosevka-extended.ttf");
pub static LOGO: &[u8] = include_bytes!("../compile/logo.png");

pub static RESOURCES: &str = "resources";

pub mod game;
pub mod render;
pub mod util;
