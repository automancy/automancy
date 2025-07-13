use std::cell::{Cell, RefCell};

use automancy_data::{
    id::{ModelId, TileId},
    math::Matrix4,
    rendering::Instance,
};
use automancy_resources::generic::DataMap;
use yakui::{paint::PaintCall, util::widget, widget::Widget, Rect, Response, Vec2};

use crate::custom::{mark_rerender, should_rerender, CustomRenderer, RenderObject};

#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
    pub instance: Instance,
    pub ty: GameObjectType,
    pub size: Vec2,
    pub model_matrix: Matrix4,
    pub world_matrix: Matrix4,
}

impl GameObject {
    pub fn new(
        instance: Instance,
        ty: GameObjectType,
        size: Vec2,
        model_matrix: Option<Matrix4>,
        world_matrix: Option<Matrix4>,
    ) -> Self {
        Self {
            instance,
            ty,
            size,
            model_matrix: model_matrix.unwrap_or_default(),
            world_matrix: world_matrix.unwrap_or_default(),
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<()> {
        widget::<GameObjectWidget>(Some(self))
    }
}

pub fn ui_game_object(
    instance: Instance,
    ty: GameObjectType,
    size: Vec2,
    model_matrix: Option<Matrix4>,
    world_matrix: Option<Matrix4>,
) -> Response<()> {
    GameObject::new(instance, ty, size, model_matrix, world_matrix).show()
}

#[derive(Debug, Clone)]
pub struct GameObjectWidget {
    props: RefCell<Option<GameObject>>,
    clip: Cell<Rect>,
}

impl Widget for GameObjectWidget {
    type Props<'a> = Option<GameObject>;
    type Response = ();

    fn new() -> Self {
        Self {
            props: RefCell::default(),
            clip: Cell::new(Rect::ZERO),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        let old = self.props.get_mut();

        if old != &props && !should_rerender() {
            mark_rerender();
        }

        *old = props;
    }

    fn layout(
        &self,
        _ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> Vec2 {
        if let Some(paint) = &*self.props.borrow() {
            constraints.constrain(paint.size)
        } else {
            constraints.min
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        if let Some(clip) = ctx.layout.get(ctx.dom.current()).map(|v| v.clip) {
            self.clip.set(clip);
        }

        let mut inside = Rect::ZERO;
        let mut clip_scale = Vec2::ONE;
        let mut clip_offset = Vec2::ZERO;
        if let Some(mut rect) = ctx.layout.get(ctx.dom.current()).map(|v| v.rect) {
            rect.set_pos(rect.pos() * ctx.layout.scale_factor());
            rect.set_size(rect.size() * ctx.layout.scale_factor());

            inside = ctx
                .layout
                .unscaled_viewport()
                .constrain(self.clip.get())
                .constrain(rect);

            clip_scale = inside.size() / rect.size();

            let inside_center = inside.pos() + inside.size() / 2.0;
            let rect_center = rect.pos() + rect.size() / 2.0;
            let sign = (inside_center - rect_center).signum();

            clip_offset = (sign + Vec2::ONE) * (Vec2::ONE - clip_scale) * rect.size();
        }

        if let Some(layer) = ctx.paint.layers.current_mut() {
            let mut props = self.props.borrow().clone().unwrap();
            props.size *= ctx.layout.scale_factor();

            let id =
                ctx.dom
                    .get_global_or_init(CustomRenderer::init)
                    .add(RenderObject::GameObject(GameObjectPaint {
                        props,
                        clip_scale,
                        clip_offset,
                    }));

            layer.calls.push((inside, PaintCall::User(id)));
        }
    }
}
