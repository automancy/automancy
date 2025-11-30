use crate::*;

#[derive(Debug, Clone, PartialEq, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct GameModel {
    pub model: GenericModel,
    pub instance: GameDrawInstance,
    pub size: Vec2,
}

impl GameModel {
    pub fn new(model: GenericModel, size: Vec2) -> GameModel {
        GameModel {
            model,
            instance: GameDrawInstance {
                world_matrix: model.view_matrix(),
                ..Default::default()
            },
            size,
        }
    }

    pub fn new_with_matrix(model: GenericModel, size: Vec2, (model_matrix, world_matrix): (math::Matrix4, math::Matrix4)) -> GameModel {
        GameModel {
            model,
            instance: GameDrawInstance {
                model_matrix,
                world_matrix,
                ..Default::default()
            },
            size,
        }
    }

    pub fn color(mut self, color_offset: PackedRgba) -> Self {
        self.instance.color_offset = color_offset;
        self
    }

    #[track_caller]
    pub fn show(self) -> Response<()> {
        widget::<GameObjectWidget>(self)
    }
}

#[derive(Debug)]
pub struct GameObjectWidget {
    props: UnsafeCell<GameModel>,
}

impl Widget for GameObjectWidget {
    type Props<'a> = GameModel;
    type Response = ();

    fn new() -> Self {
        Self {
            props: Default::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        *self.props.get_mut() = props;
    }

    fn layout(&self, ctx: yakui::widget::LayoutContext<'_>, constraints: yakui::Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        let props = unsafe { &*self.props.get() };

        constraints.constrain(props.size)
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let layout_node = ctx.layout.get(ctx.dom.current()).unwrap();

        let mut clip_rect = Rect::from_pos_size(layout_node.clip.pos(), layout_node.clip.size());
        clip_rect.set_pos(clip_rect.pos() * ctx.layout.scale_factor());
        clip_rect.set_size(clip_rect.size() * ctx.layout.scale_factor());

        let mut layout_rect = Rect::from_pos_size(layout_node.rect.pos(), layout_node.rect.size());
        layout_rect.set_pos(layout_rect.pos() * ctx.layout.scale_factor());
        layout_rect.set_size(layout_rect.size() * ctx.layout.scale_factor());

        if let Some(layer) = ctx.paint.layers.current_mut() {
            const SUPER_SAMPLE_FACTOR: f32 = 2.0 + (2.0 / 3.0);

            let props: GameModel = std::mem::take(unsafe { &mut *self.props.get() });

            let (texture_id, rect) = ctx.paint.globals.get_mut().get_mut(custom::CustomRenderer::default).add(
                layout_rect,
                props.size.round() * SUPER_SAMPLE_FACTOR * ctx.layout.scale_factor(),
                props.into(),
            );

            shapes::PaintRect::new(layout_rect).texture((texture_id, rect)).add(ctx.paint);
        }
    }
}
