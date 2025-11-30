use num_traits::{AsPrimitive, Num};

use crate::*;

const TRACK_HEIGHT: f32 = 8.0;
const KNOB_SIZE: f32 = 16.0;
const TOTAL_HEIGHT: f32 = KNOB_SIZE * 2.5;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Slider<T> {
    pub value: T,
    pub min: T,
    pub max: T,
    pub step: Option<T>,
}

impl<T> Slider<T>
where
    T: Num + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    pub fn new(value: T, min: T, max: T) -> Self {
        Slider {
            value,
            min,
            max,
            step: None,
        }
    }

    pub fn new_with_range(value: T, range: RangeInclusive<T>) -> Self {
        Slider {
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
    pub fn show(self) -> Response<SliderResponse<T>> {
        widget::<SliderWidget<T>>(self)
    }
}

#[derive(Debug)]
pub struct SliderResponse<T> {
    pub value: Option<T>,
}

#[derive(Debug)]
pub struct SliderWidget<T> {
    props: Slider<T>,
    value: T,
    dragging: bool,
    value_changed: bool,
    rect: Cell<Option<Rect>>,
}

impl<T> SliderWidget<T>
where
    T: Num + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    fn update_value(&mut self, pos: f32) {
        if let Some(rect) = self.rect.get() {
            let min_pos = rect.pos().x;
            let max_pos = rect.pos().x + rect.size().x;

            let pos = pos.clamp(min_pos, max_pos);

            let percentage = (pos - min_pos) / (max_pos - min_pos);
            let min = self.props.min.as_();
            let max = self.props.max.as_();

            self.value = (percentage.mul_add(max - min, min)).as_();
        }
        self.value_changed = true;
    }
}

impl<T> Widget for SliderWidget<T>
where
    T: Num + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    type Props<'a> = Slider<T>;
    type Response = SliderResponse<T>;

    fn new() -> Self {
        Self {
            props: Slider::new(T::zero(), T::zero(), T::one()),
            value: T::zero(),
            dragging: false,
            value_changed: false,
            rect: Cell::new(None),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        if !self.value_changed {
            self.value = props.value;
        }
        self.value_changed = false;
        self.props = props;

        if let Some(step) = self.props.step {
            self.value = T::round_to_step(self.value, step);
        }

        if self.value != self.props.value {
            SliderResponse {
                value: Some(self.value),
            }
        } else {
            SliderResponse {
                value: None,
            }
        }
    }

    fn layout(&self, _ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        Vec2::new(
            constraints.constrain_width(KNOB_SIZE * 6.0) + KNOB_SIZE * 4.0,
            constraints.min.y.max(TOTAL_HEIGHT),
        )
    }

    fn paint(&self, ctx: PaintContext<'_>) {
        let layout = ctx.layout.get(ctx.dom.current()).unwrap();

        let rect = Rect::from_pos_size(
            layout.rect.pos() + Vec2::new(KNOB_SIZE, 0.0),
            layout.rect.size() - Vec2::new(KNOB_SIZE * 2.0, 0.0),
        );
        self.rect.set(Some(rect));

        shapes::PaintRoundRect::new(
            Rect::from_pos_size(
                rect.pos() + Vec2::new(0.0, (TOTAL_HEIGHT - TRACK_HEIGHT) / 2.0),
                Vec2::new(rect.size().x, TRACK_HEIGHT),
            ),
            0.0,
        )
        .color(colors::BACKGROUND_2.yak())
        .add(ctx.paint);

        let min = self.props.min.as_();
        let max = self.props.max.as_();
        let value = self.props.value.as_();
        let percentage = (value - min) / (max - min);

        let percentage = percentage.clamp(0.0, 1.0);

        let knob_pos = Vec2::new(
            (rect.size().x - KNOB_SIZE) * percentage + ((percentage * 2.0 - 1.0) * KNOB_SIZE / 2.0),
            (TOTAL_HEIGHT - KNOB_SIZE) / 2.0,
        );

        shapes::PaintRoundRect::new(
            Rect::from_pos_size(rect.pos() + knob_pos, Vec2::new(KNOB_SIZE, KNOB_SIZE)),
            KNOB_SIZE / 2.0,
        )
        .color(colors::INTERACTIVE_2.yak())
        .add(ctx.paint);
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match event {
            WidgetEvent::MouseLeave => {
                self.dragging = false;

                EventResponse::Bubble
            },
            WidgetEvent::MouseMoved(position) => {
                if self.dragging
                    && let Some(position) = *position
                {
                    self.update_value(position.x);

                    EventResponse::Sink
                } else {
                    EventResponse::Bubble
                }
            },
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                position,
                ..
            } => {
                self.dragging = false;

                if *inside && *down {
                    self.dragging = true;
                    self.update_value(position.x);

                    EventResponse::Sink
                } else {
                    EventResponse::Bubble
                }
            },
            _ => EventResponse::Bubble,
        }
    }
}

#[track_caller]
pub fn slider_with_input<T>(value: T, range: RangeInclusive<T>, step: Option<T>) -> Option<T>
where
    T: Num + FromStr + ToString + RoundExt + Debug + AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    let mut result = None;

    row_cross_center(|| {
        if let Some(v) = Slider::new_with_range(result.unwrap_or(value), range.clone())
            .step(step)
            .show()
            .value
        {
            result = Some(v);
        }

        Pad::none().left(sizing::PADDING_SMALL).show(|| {
            if let Some(v) = NumberInput::new_with_range(result.unwrap_or(value), range).step(step).show().value {
                result = Some(v);
            }
        });
    });

    result
}
