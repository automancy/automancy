use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Margin {
    pub size: Vec2,
}

impl Margin {
    pub fn new(size: Vec2) -> Self {
        Self {
            size,
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<MarginResponse> {
        widget::<MarginWidget>(self)
    }
}

#[derive(Debug)]
pub struct MarginWidget {
    props: Margin,
}

pub type MarginResponse = ();

impl Widget for MarginWidget {
    type Props<'a> = Margin;
    type Response = MarginResponse;

    fn new() -> Self {
        Self {
            props: Margin::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, _ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        constraints.constrain(self.props.size)
    }
}
