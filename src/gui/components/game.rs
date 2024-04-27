use std::{cell::Cell, time::Instant};

use automancy_defs::{bytemuck, glam::vec3, id::Id, math::Matrix4, rendering::InstanceData};
use wgpu::IndexFormat;
use yakui::{paint::PaintCall, util::widget, widget::Widget, Rect, Response, Vec2};
use yakui_wgpu::CallbackTrait;

use crate::{gpu, gui::YakuiRenderResources, renderer::try_add_animation};

thread_local! {
    static START_INSTANT: Cell<Option<Instant>> = const { Cell::new(None) };
    static INDEX_COUNTER: Cell<usize> = const { Cell::new(0) };
}

pub fn init_custom_paint_state(start_instant: Instant) {
    START_INSTANT.set(Some(start_instant));
}

pub fn reset_custom_paint_state() {
    INDEX_COUNTER.replace(0);
}

#[derive(Debug, Clone, Copy)]
pub struct GameElement {
    instance: InstanceData,
    model: Id,
    index: usize,
    size: Vec2,
}

pub fn ui_game_object(instance: InstanceData, model: Id, size: Vec2) -> Response<Option<Rect>> {
    GameElement::new(instance, model, size).show()
}

impl GameElement {
    pub fn new(instance: InstanceData, model: Id, size: Vec2) -> Self {
        let index = INDEX_COUNTER.get();

        let result = Self {
            instance,
            model,
            index,
            size,
        };
        INDEX_COUNTER.set(index + 1);

        result
    }

    pub fn show(self) -> Response<Option<Rect>> {
        widget::<GameElementWidget>(Some(self))
    }
}

#[derive(Debug, Clone)]
pub struct GameElementWidget {
    paint: Cell<Option<GameElement>>,
    layout_rect: Cell<Option<Rect>>,
    clip: Cell<Rect>,
    adjusted_matrix: Cell<Option<Matrix4>>,
}

impl CallbackTrait<YakuiRenderResources> for GameElementWidget {
    fn prepare(
        &self,
        (
        resource_man,
        _global_buffers,
        _gui_resources,
        animation_map,
        instances,
        _draws,
    ): &mut YakuiRenderResources,
    ) {
        if let Some(mut paint) = self.paint.get() {
            let start_instant = START_INSTANT.get().unwrap();
            try_add_animation(resource_man, start_instant, paint.model, animation_map);

            if let Some(m) = self.adjusted_matrix.get() {
                paint.instance = paint.instance.with_world_matrix(m);
            }

            instances
                .as_mut()
                .unwrap()
                .push((paint.instance, paint.model, paint.index));
        }
    }

    fn finish_prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (
            resource_man,
            _global_buffers,
            gui_resources,
            animation_map,
            instances,
            draws,
        ): &mut YakuiRenderResources,
    ) {
        if let Some(mut instances) = instances.take() {
            let gui_resources = gui_resources.as_mut().unwrap();

            instances.sort_by_key(|v| v.1);

            let (instances, draws_result, _count, matrix_data) =
                gpu::indirect_instance(resource_man, &instances, false, animation_map);

            gpu::create_or_write_buffer(
                device,
                queue,
                &mut gui_resources.instance_buffer,
                bytemuck::cast_slice(instances.as_slice()),
            );

            queue.write_buffer(
                &gui_resources.matrix_data_buffer,
                0,
                bytemuck::cast_slice(matrix_data.as_slice()),
            );

            *draws = draws_result;
        }
    }

    fn paint<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        (
            _resource_man,
            global_buffers,
            gui_resources,
            _animation_map,
            _instances,
            draws,
        ): &'a YakuiRenderResources,
    ) {
        let gui_resources = gui_resources.as_ref().unwrap();

        render_pass.set_pipeline(&gui_resources.pipeline);
        render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
        render_pass.set_vertex_buffer(0, global_buffers.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, gui_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(global_buffers.index_buffer.slice(..), IndexFormat::Uint16);

        let clip = self.clip.get();

        if clip.size().x > 0.0 && clip.size().y > 0.0 && clip.pos().x >= 0.0 && clip.pos().y >= 0.0
        {
            render_pass.set_viewport(
                clip.pos().x,
                clip.pos().y,
                clip.size().x,
                clip.size().y,
                0.0,
                1.0,
            );
        }

        for (draw, ..) in draws[&self.paint.get().unwrap().model]
            .iter()
            .filter(|v| v.1 == self.paint.get().unwrap().index)
        {
            render_pass.draw_indexed(
                draw.first_index..(draw.first_index + draw.index_count),
                draw.base_vertex,
                draw.first_instance..(draw.first_instance + draw.instance_count),
            );
        }

        {
            render_pass.set_pipeline(&gui_resources.depth_clear_pipeline);
            render_pass.draw(0..3, 0..1);
        }
    }
}

impl Widget for GameElementWidget {
    type Props<'a> = Option<GameElement>;
    type Response = Option<Rect>;

    fn new() -> Self {
        Self {
            paint: Cell::default(),
            layout_rect: Cell::default(),
            clip: Cell::new(Rect::ZERO),
            adjusted_matrix: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.paint.set(props);

        self.layout_rect.get()
    }

    fn layout(
        &self,
        ctx: yakui::widget::LayoutContext<'_>,
        _constraints: yakui::Constraints,
    ) -> yakui::Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        if let Some(layout_node) = ctx.layout.get(ctx.dom.current()) {
            self.layout_rect.set(Some(layout_node.rect));
        }

        if let Some(paint) = self.paint.get() {
            paint.size
        } else {
            Vec2::ZERO
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let clip = ctx.paint.get_current_clip();

        if let Some((paint, layout_rect)) = self.paint.get().zip(self.layout_rect.get()) {
            let clip = self.clip.get();

            if clip.size().x > 0.0 && clip.size().y > 0.0 {
                let mut rect = layout_rect;
                rect.set_size(rect.size() * ctx.layout.scale_factor());
                rect.set_pos(rect.pos() * ctx.layout.scale_factor());

                let inside = clip.constrain(rect);
                if !inside.size().abs_diff_eq(Vec2::ZERO, 0.1) {
                    let sign =
                        (rect.max() - rect.size() / 2.0) - (inside.max() - inside.size() / 2.0);

                    let sx = rect.size().x / inside.size().x;
                    let sy = rect.size().y / inside.size().y;

                    let dx = (sx - 1.0) * sign.x.signum();
                    let dy = (sy - 1.0) * sign.y.signum();

                    self.adjusted_matrix.set(Some(
                        Matrix4::from_translation(vec3(dx, dy, 0.0))
                            * paint
                                .instance
                                .get_world_matrix()
                                .unwrap_or(Matrix4::IDENTITY)
                            * Matrix4::from_scale(vec3(sx, sy, 1.0)),
                    ));
                }
            }
        }

        if let Some(clip) = clip {
            self.clip.set(clip);
        }

        if let Some(layer) = ctx.paint.layers_mut().current_mut() {
            layer
                .calls
                .push((PaintCall::Custom(yakui_wgpu::cast(self.clone())), clip));
        }
    }
}
