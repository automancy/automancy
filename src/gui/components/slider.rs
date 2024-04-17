use std::{cell::Cell, fmt::Debug, ops::RangeInclusive};

use automancy_defs::colors;
use num::Num;
use yakui::{
    colored_box, colored_circle, draggable,
    util::widget,
    widget::{LayoutContext, PaintContext, Widget},
    Color, Constraints, Rect, Response, Vec2,
};

use crate::util::round::Round;

const TRACK_COLOR: Color = colors::BACKGROUND_2;
const KNOB_COLOR: Color = colors::ORANGE;

const DEFAULT_WIDTH: f32 = 150.0;
const TRACK_HEIGHT: f32 = 8.0;
const KNOB_SIZE: f32 = 16.0;
const TOTAL_HEIGHT: f32 = KNOB_SIZE * 1.5;

#[derive(Debug)]
#[non_exhaustive]
pub struct Slider<T: Copy> {
    pub value: T,
    pub min: T,
    pub max: T,
    pub step: Option<T>,
}

impl<T: 'static + Num + Debug + Round + Copy> Slider<T> {
    pub fn new(value: T, min: T, max: T) -> Self {
        Slider {
            value,
            min,
            max,
            step: None,
        }
    }

    pub fn show(self) -> Response<SliderResponse<T>> {
        widget::<SliderWidget<T>>(self)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct SliderResponse<T> {
    pub value: Option<T>,
}

#[derive(Debug)]
pub struct SliderWidget<T: Copy> {
    props: Slider<T>,
    rect: Cell<Option<Rect>>,
}

impl<T: 'static + Num + Debug + Round + Copy> Widget for SliderWidget<T> {
    type Props<'a> = Slider<T>;
    type Response = SliderResponse<T>;

    fn new() -> Self {
        Self {
            props: Slider::new(T::zero(), T::zero(), T::one()),
            rect: Cell::new(None),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        colored_box(TRACK_COLOR, [0.0, TRACK_HEIGHT]);
        let res = draggable(|| {
            colored_circle(KNOB_COLOR, KNOB_SIZE);
        });

        let mut value = self.props.value;

        if let (Some(drag), Some(rect)) = (res.dragging, self.rect.get()) {
            let min_pos = rect.pos().x;
            let max_pos = rect.pos().x + rect.size().x - KNOB_SIZE;
            let actual_pos = drag.current.x.clamp(min_pos, max_pos);

            let percentage = ((actual_pos - min_pos) / (max_pos - min_pos)) as f64;
            let min = self.props.min.to_f64();
            let max = self.props.max.to_f64();

            value = T::from_f64(min + percentage * (max - min));
        }

        if let Some(step) = self.props.step {
            value = round_to_step(value, step);
        }

        if value != self.props.value {
            SliderResponse { value: Some(value) }
        } else {
            SliderResponse { value: None }
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();
        let size = Vec2::new(
            constraints.constrain_width(DEFAULT_WIDTH).max(KNOB_SIZE),
            constraints.min.y.max(TOTAL_HEIGHT),
        );

        let track = node.children[0];
        let knob = node.children[1];

        let track_constraints = Constraints::tight(Vec2::new(size.x - KNOB_SIZE, TRACK_HEIGHT));
        ctx.calculate_layout(track, track_constraints);
        ctx.layout.set_pos(
            track,
            Vec2::new(KNOB_SIZE / 2.0, (TOTAL_HEIGHT - TRACK_HEIGHT) / 2.0),
        );

        let min = self.props.min.to_f64();
        let max = self.props.max.to_f64();
        let value = self.props.value.to_f64();
        let percentage = (value - min) / (max - min);

        let percentage = percentage.clamp(0.0, 1.0);

        let knob_offset = (size.x - KNOB_SIZE) * percentage as f32;
        let knob_pos = Vec2::new(knob_offset, (TOTAL_HEIGHT - KNOB_SIZE) / 2.0);
        ctx.calculate_layout(knob, Constraints::none());
        ctx.layout.set_pos(knob, knob_pos);

        size
    }

    fn paint(&self, mut ctx: PaintContext<'_>) {
        // This is a little gross: stash our position from this frame's layout
        // pass so that we can compare it against any drag updates that happen
        // at the beginning of the next frame.
        let layout = ctx.layout.get(ctx.dom.current()).unwrap();
        self.rect.set(Some(layout.rect));

        let node = ctx.dom.get_current();
        for &child in &node.children {
            ctx.paint(child);
        }
    }
}

fn round_to_step<T: Num + Round + Copy>(value: T, step: T) -> T {
    if step == T::zero() {
        value
    } else {
        (value / step).generic_round() * step
    }
}

pub fn slider<T: 'static + Num + Debug + Round + Copy>(
    value: &mut T,
    range: RangeInclusive<T>,
    step: Option<T>,
) {
    let mut slider = Slider::new(*value, *range.start(), *range.end());
    slider.step = step;

    if let Some(v) = slider.show().value {
        *value = v
    }
}
