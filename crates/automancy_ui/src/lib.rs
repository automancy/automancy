use automancy_defs::math::Float;

pub static SYMBOLS_FONT: &[u8] = include_bytes!("assets/SymbolsNerdFont-Regular.ttf");
pub static SYMBOLS_FONT_KEY: &str = "Symbols Nerd Font Mono";

pub const TINY_ICON_SIZE: Float = 16.0;
pub const SMALL_ICON_SIZE: Float = 24.0;
pub const MEDIUM_ICON_SIZE: Float = 48.0;
pub const LARGE_ICON_SIZE: Float = 96.0;

pub const ROUNDED_MEDIUM: f32 = 6.0;

mod components;
pub use self::components::*;
