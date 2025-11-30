use include_dir::{Dir, include_dir};

use crate::*;

pub static SYMBOLS_FONT_LICENSE: &str = include_str!("assets/SymbolsNerdFont-LICENSE.txt");
pub static SYMBOLS_FONT_KEY: &str = "Symbols Nerd Font Mono";
pub static SYMBOLS_FONT: &[u8] = include_bytes!("assets/SymbolsNerdFontMono-Regular.ttf");

pub static MONOSPACE_FONT_LICENSE: &str = include_str!("assets/CommitMono-LICENSE.txt");
pub static MONOSPACE_FONT_KEY: &str = "CommitMono";
pub static MONOSPACE_FONT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/CommitMono");

pub fn load_static_fonts(fonts: &Fonts) {
    log::info!("Loading static fonts...");

    log::info!("Loading static font {SYMBOLS_FONT_KEY}.");
    fonts.load_font_source(cosmic_text::fontdb::Source::Binary(Arc::new(&SYMBOLS_FONT)));

    log::info!("Loading static font {MONOSPACE_FONT_KEY}.");
    for file in MONOSPACE_FONT_DIR.files() {
        fonts.load_font_source(cosmic_text::fontdb::Source::Binary(Arc::new(file.contents())));
    }

    fonts.set_sans_serif_family("");
    fonts.set_monospace_family(MONOSPACE_FONT_KEY);

    log::info!("Static fonts loaded.");
}
