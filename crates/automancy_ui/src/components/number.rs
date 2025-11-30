use num_traits::{AsPrimitive, Num};

use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct NumberInput<T> {
    pub value: T,
    pub min: T,
    pub max: T,
    pub step: Option<T>,
}

impl<T> NumberInput<T>
where
    T: Num + FromStr + ToString + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    pub fn new(value: T, min: T, max: T) -> Self {
        NumberInput {
            value,
            min,
            max,
            step: None,
        }
    }

    pub fn new_with_range(value: T, range: RangeInclusive<T>) -> Self {
        NumberInput {
            value,
            min: *range.start(),
            max: *range.end(),
            step: None,
        }
    }

    pub fn step(self, step: Option<T>) -> Self {
        Self {
            step,
            ..self
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<NumberInputResponse<T>> {
        widget::<NumberInputWidget<T>>(self)
    }
}

#[derive(Debug)]
pub struct NumberInputResponse<T> {
    pub value: Option<T>,
}

#[derive(Debug)]
pub struct NumberInputWidget<T: Copy> {
    props: NumberInput<T>,
    value: Cell<T>,
    max_text_len: Cell<usize>,
    max_text_width: Cell<f32>,
    pause_update: Cell<bool>,
    text: RefCell<String>,
    style: TextStyle,
}

impl<T> Widget for NumberInputWidget<T>
where
    T: Num + FromStr + ToString + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    type Props<'a> = NumberInput<T>;
    type Response = NumberInputResponse<T>;

    fn new() -> Self {
        Self {
            props: NumberInput::new(T::zero(), T::zero(), T::one()),
            value: Cell::new(T::zero()),
            max_text_len: Default::default(),
            max_text_width: Default::default(),
            pause_update: Default::default(),
            text: Default::default(),
            style: TextStyle::mono().align(TextAlignment::End),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        if self.max_text_len.get() == 0 || self.props.max != props.max {
            let max_text = props.max.to_string();

            self.max_text_len.set(max_text.len());

            let fonts = context::dom().get_global_or_init(Fonts::default);
            let max_width = fonts.with_inner(|fonts| {
                measure_text_width(&mut fonts.font_system, &mut fonts.font_selection, &max_text, &self.style, 1.0, None)
            });

            self.max_text_width.set(max_width);
        }

        self.value.set(props.value);
        self.props = props;

        if !self.pause_update.get() {
            self.text.replace(self.value.get().to_string());
        }

        let mut response = TextBox::normal()
            .placeholder(self.value.get().to_string())
            .placeholder_style(self.style.clone().color(colors::TEXT_INACTIVE.yak()))
            .style(self.style.clone())
            .min_width(self.max_text_width.get())
            .show(self.text.borrow().as_str());

        if let Some(new_text) = response.text.take()
            && new_text.len() <= self.max_text_len.get()
        {
            self.text
                .replace(new_text.into_chars().filter(|char| char.is_ascii_digit()).collect::<String>());
        }

        self.pause_update.set(response.active);

        if (response.activated || response.lost_focus)
            && let Ok(v) = self.text.borrow().as_str().trim().parse::<T>()
        {
            self.value.set(v.clamp(self.props.min, self.props.max));
        }

        if let Some(step) = self.props.step {
            self.value.set(T::round_to_step(self.value.get(), step));
        }

        if self.value.get() != self.props.value {
            NumberInputResponse {
                value: Some(self.value.get()),
            }
        } else {
            NumberInputResponse {
                value: None,
            }
        }
    }

    fn layout(&self, ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        self.default_layout(
            ctx,
            Constraints::loose(constraints.max.min(Vec2::new(self.max_text_width.get(), f32::INFINITY))),
        )
    }
}
