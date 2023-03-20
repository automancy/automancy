#![feature(slice_group_by)]
#![feature(duration_consts_float)]
#![feature(result_option_inspect)]
#![feature(arc_unwrap_or_clone)]
#![feature(is_some_and)]
#![feature(step_trait)]

pub static IOSEVKA_FONT: &[u8] = include_bytes!("../compile/fonts/iosevka-extended.ttf");
pub static LOGO: &[u8] = include_bytes!("../compile/logo.png");

pub mod game;
pub mod render;
pub mod resource;
pub mod util;
