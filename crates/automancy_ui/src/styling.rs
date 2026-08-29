use crate::*;

pub const trait TextStyleExt {
    #[must_use]
    fn normal() -> TextStyle;
    #[must_use]
    fn small() -> TextStyle;
    #[must_use]
    fn heading() -> TextStyle;

    #[must_use]
    fn mono() -> TextStyle;
    #[must_use]
    fn symbols() -> TextStyle;
}

const FONT_KEY_SANS_SERIF: FontKey = FontKey {
    family: FontFamily::SansSerif,
    stretch: FontWidth::Normal,
    style: FontStyle::Normal,
    weight: FontWeight::MEDIUM,
};

const FONT_KEY_SANS_SERIF_LARGE: FontKey = FontKey {
    family: FontFamily::SansSerif,
    stretch: FontWidth::Normal,
    style: FontStyle::Normal,
    weight: FontWeight::SEMIBOLD,
};

const FONT_KEY_MONOSPACE: FontKey = FontKey {
    family: FontFamily::Monospace,
    stretch: FontWidth::Normal,
    style: FontStyle::Normal,
    weight: FontWeight::MEDIUM,
};

const FONT_KEY_SYMBOLS: FontKey = FontKey {
    family: FontFamily::Name(Cow::Borrowed(crate::static_fonts::SYMBOLS_FONT_KEY)),
    stretch: FontWidth::Normal,
    style: FontStyle::Normal,
    weight: FontWeight::NORMAL,
};

const impl TextStyleExt for TextStyle {
    fn normal() -> TextStyle {
        TextStyle {
            font_size: sizing::NORMAL_TEXT,
            font: FONT_KEY_SANS_SERIF,
            line_height_override: None,
            color: colors::TEXT_ACTIVE.yak(),
            align: TextAlignment::Start,
        }
    }

    fn small() -> TextStyle {
        TextStyle {
            font_size: sizing::SMALL_TEXT,
            font: FONT_KEY_SANS_SERIF,
            line_height_override: None,
            color: colors::TEXT_ACTIVE.yak(),
            align: TextAlignment::Start,
        }
    }

    fn heading() -> TextStyle {
        TextStyle {
            font_size: sizing::HEADING_TEXT,
            font: FONT_KEY_SANS_SERIF_LARGE,
            line_height_override: None,
            color: colors::TEXT_ACTIVE.yak(),
            align: TextAlignment::Start,
        }
    }

    fn mono() -> TextStyle {
        TextStyle {
            font_size: sizing::NORMAL_TEXT,
            font: FONT_KEY_MONOSPACE,
            line_height_override: None,
            color: colors::TEXT_ACTIVE.yak(),
            align: TextAlignment::Start,
        }
    }

    fn symbols() -> TextStyle {
        TextStyle {
            font_size: sizing::SMALL_TEXT,
            font: FONT_KEY_SYMBOLS,
            line_height_override: None,
            color: colors::TEXT_ACTIVE.yak(),
            align: TextAlignment::Start,
        }
    }
}
