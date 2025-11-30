use crate::*;

#[track_caller]
pub fn symbol<S: Into<Cow<'static, str>>>(symbol: S, color: Color) {
    constrained(
        Constraints::tight(Vec2::new(sizing::NORMAL_TEXT + sizing::PADDING_MEDIUM, sizing::NORMAL_TEXT)),
        || {
            align(Alignment::CENTER, || {
                Text::symbol(symbol).color(color).show();
            });
        },
    );
}

pub trait TextExt {
    fn small<S: Into<Cow<'static, str>>>(text: S) -> Text;
    fn normal<S: Into<Cow<'static, str>>>(text: S) -> Text;
    fn heading<S: Into<Cow<'static, str>>>(text: S) -> Text;
    fn mono<S: Into<Cow<'static, str>>>(text: S) -> Text;
    fn symbol<S: Into<Cow<'static, str>>>(symbol: S) -> Text;

    fn color(self, color: Color) -> Text;
    fn align(self, align: TextAlignment) -> Text;
}

impl TextExt for Text {
    fn small<S: Into<Cow<'static, str>>>(text: S) -> Text {
        Text::with_style(text, TextStyle::small()).padding(Pad::all(sizing::PADDING_SMALL))
    }

    fn normal<S: Into<Cow<'static, str>>>(text: S) -> Text {
        Text::with_style(text, TextStyle::normal()).padding(Pad::all(sizing::PADDING_SMALL))
    }

    fn heading<S: Into<Cow<'static, str>>>(text: S) -> Text {
        Text::with_style(text, TextStyle::heading()).padding(Pad::all(sizing::PADDING_MEDIUM))
    }

    fn mono<S: Into<Cow<'static, str>>>(text: S) -> Text {
        Text::with_style(text, TextStyle::mono()).padding(Pad::all(sizing::PADDING_SMALL))
    }

    fn symbol<S: Into<Cow<'static, str>>>(symbol: S) -> Text {
        Text::with_style(symbol, TextStyle::symbols())
    }

    fn color(mut self, color: Color) -> Text {
        self.style.color = color;
        self
    }

    fn align(mut self, align: TextAlignment) -> Text {
        self.style.align = align;
        self
    }
}
