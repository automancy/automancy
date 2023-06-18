pub static IOSEVKA_FONT: &[u8] = include_bytes!("../compile/fonts/iosevka-extended.ttf");
pub static VERSION: &str = include_str!("../compile/version.txt");

pub mod game;
pub mod render;
pub mod util;
