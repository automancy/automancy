use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

use automancy_defs::{
    glam::vec3,
    id::Id,
    math::Matrix4,
    rendering::{InstanceData, PostProcessingUBO},
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferUsages, Color, Extent3d,
    IndexFormat, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureUsages,
    TextureViewDescriptor,
};
use yakui::{paint::PaintCall, util::widget, widget::Widget, Rect, Response, Vec2};
use yakui_wgpu::CallbackTrait;

use crate::{
    gpu::{self, DEPTH_FORMAT, MODEL_FORMAT, NORMAL_CLEAR, NORMAL_FORMAT},
    gui::YakuiRenderResources,
    renderer::try_add_animation,
};

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

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug)]
pub struct GameElementPaint {
    repaint: bool,
    props: GameElement,
    clip: Rect,
    adjusted_matrix: Option<Matrix4>,

    post_processing_texture: Option<Texture>,
    present_bind_group: Option<BindGroup>,
}

impl CallbackTrait<YakuiRenderResources> for GameElementPaint {
    fn prepare(
        &mut self,
        (
            resource_man,
            _global_resources,
            _gui_resources,
            _surface_format,
            animation_map,
            instances,
            _draws,
        ): &mut YakuiRenderResources,
    ) {
        let mut props = self.props;
        let start_instant = START_INSTANT.get().unwrap();
        if try_add_animation(resource_man, start_instant, props.model, animation_map) {
            self.repaint = true;
        }

        if let Some(m) = self.adjusted_matrix {
            props.instance = props.instance.with_world_matrix(m);
        }

        instances
            .as_mut()
            .unwrap()
            .push((props.instance, props.model, props.index));
    }

    fn finish_prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        (
            resource_man,
            global_resources,
            gui_resources,
            surface_format,
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

        let mut clip = self.clip;
        if clip.pos().x < 0.0 || clip.pos().y < 0.0 || clip.size().x < 1.0 || clip.size().y < 1.0 {
            return;
        }
        clip.set_size(clip.size() * 2.0);

        if self.post_processing_texture.is_none() {
            self.post_processing_texture = Some(device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: clip.size().x as u32,
                    height: clip.size().y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: *surface_format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }

        if self.present_bind_group.is_none() {
            self.present_bind_group = Some(
                device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &global_resources.intermediate_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(
                                &self
                                    .post_processing_texture
                                    .as_ref()
                                    .unwrap()
                                    .create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&global_resources.filtering_sampler),
                        },
                    ],
                }),
            )
        }

        if self.repaint {
            self.repaint = false;

            let color_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: clip.size().x as u32,
                    height: clip.size().y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: *surface_format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let depth_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: clip.size().x as u32,
                    height: clip.size().y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let model_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: clip.size().x as u32,
                    height: clip.size().y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: MODEL_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let normal_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: clip.size().x as u32,
                    height: clip.size().y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: NORMAL_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let post_processing_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[PostProcessingUBO {
                    world_matrix: Matrix4::IDENTITY.to_cols_array_2d(),
                }]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            let post_processing_bind_group_uniform =
                device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &global_resources.post_processing_bind_group_layout_uniform,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: post_processing_uniform_buffer.as_entire_binding(),
                    }],
                });

            let color = color_texture.create_view(&TextureViewDescriptor::default());
            let depth = depth_texture.create_view(&TextureViewDescriptor::default());
            let model = model_texture.create_view(&TextureViewDescriptor::default());
            let normal = normal_texture.create_view(&TextureViewDescriptor::default());

            let post_processing_bind_group_textures =
                device.create_bind_group(&BindGroupDescriptor {
                    layout: &global_resources.post_processing_bind_group_layout_textures,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::Sampler(&global_resources.filtering_sampler),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(
                                &global_resources.nonfiltering_sampler,
                            ),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&global_resources.repeating_sampler),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(&color),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(&normal),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: BindingResource::TextureView(&model),
                        },
                        BindGroupEntry {
                            binding: 6,
                            resource: BindingResource::TextureView(
                                &global_resources
                                    .ssao_noise_map
                                    .create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                    ],
                    label: None,
                });

            let gui_resources = gui_resources.as_ref().unwrap();

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some(&format!("UI Model Render Pass {:?}", self.props.model)),
                    color_attachments: &[
                        Some(RenderPassColorAttachment {
                            view: &color,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        }),
                        Some(RenderPassColorAttachment {
                            view: &normal,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(NORMAL_CLEAR),
                                store: StoreOp::Store,
                            },
                        }),
                        Some(RenderPassColorAttachment {
                            view: &model,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &depth,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });

                render_pass.set_pipeline(&global_resources.game_pipeline);
                render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
                render_pass.set_vertex_buffer(0, global_resources.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, gui_resources.instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(global_resources.index_buffer.slice(..), IndexFormat::Uint16);

                for (draw, ..) in draws[&self.props.model]
                    .iter()
                    .filter(|v| v.1 == self.props.index)
                {
                    render_pass.draw_indexed(
                        draw.first_index..(draw.first_index + draw.index_count),
                        draw.base_vertex,
                        draw.first_instance..(draw.first_instance + draw.instance_count),
                    );
                }
            }

            {
                let view = self
                    .post_processing_texture
                    .as_ref()
                    .unwrap()
                    .create_view(&TextureViewDescriptor::default());
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some(&format!(
                        "UI Model Post Processing Render Pass {:?}",
                        self.props.model
                    )),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&global_resources.post_processing_pipeline);
                render_pass.set_bind_group(0, &post_processing_bind_group_textures, &[]);
                render_pass.set_bind_group(1, &post_processing_bind_group_uniform, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }
    }

    fn paint<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        (
            _resource_man,
            global_resources,
            _gui_resources,
            _surface_format,
            _animation_map,
            _instances,
            _draws,
        ): &'a YakuiRenderResources,
    ) {
        if let Some(present) = &self.present_bind_group {
            let clip = self.clip;
            if clip.size().x > 0.0
                && clip.size().y > 0.0
                && clip.pos().x >= 0.0
                && clip.pos().y >= 0.0
            {
                render_pass.set_viewport(
                    clip.pos().x.round(),
                    clip.pos().y.round(),
                    clip.size().x.round(),
                    clip.size().y.round(),
                    0.0,
                    1.0,
                );
            }

            render_pass.set_pipeline(&global_resources.present_pipeline);
            render_pass.set_bind_group(0, present, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameElementPaintRef {
    inner: Rc<RefCell<GameElementPaint>>,
}

impl CallbackTrait<YakuiRenderResources> for GameElementPaintRef {
    fn prepare(&mut self, custom_resources: &mut YakuiRenderResources) {
        self.inner.borrow_mut().prepare(custom_resources);
    }

    fn finish_prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        custom_resources: &mut YakuiRenderResources,
    ) {
        self.inner
            .borrow_mut()
            .finish_prepare(device, queue, encoder, custom_resources);
    }

    fn paint<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        custom_resources: &'a YakuiRenderResources,
    ) {
        // SAFETY: the Rc will live as long as the wrapper does
        unsafe {
            self.inner.as_ptr().as_ref().unwrap().paint(
                render_pass,
                device,
                queue,
                custom_resources,
            )
        };
    }
}

#[derive(Debug, Clone)]
pub struct GameElementWidget {
    props: Cell<Option<GameElement>>,
    layout_rect: Cell<Option<Rect>>,
    clip: Cell<Rect>,
    paint: RefCell<Option<GameElementPaintRef>>,
}

impl Widget for GameElementWidget {
    type Props<'a> = Option<GameElement>;
    type Response = Option<Rect>;

    fn new() -> Self {
        Self {
            props: Cell::default(),
            layout_rect: Cell::default(),
            clip: Cell::new(Rect::ZERO),
            paint: RefCell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        if self.props.get() != props {
            *self.paint.borrow_mut() = None;
        }
        self.props.set(props);

        self.layout_rect.get()
    }

    fn layout(
        &self,
        ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> yakui::Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        if let Some(layout_node) = ctx.layout.get(ctx.dom.current()) {
            let last = self.layout_rect.get();
            if last != Some(layout_node.rect) {
                *self.paint.borrow_mut() = None;
            }
            self.layout_rect.set(Some(layout_node.rect));
        }

        if let Some(paint) = self.props.get() {
            constraints.constrain(paint.size)
        } else {
            constraints.min
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let clip = ctx.paint.get_current_clip();

        let mut r = self.paint.borrow_mut();

        let paint = r.get_or_insert_with(|| GameElementPaintRef {
            inner: Rc::new(RefCell::new(GameElementPaint {
                repaint: true,
                props: self.props.get().unwrap(),
                clip: Rect::ZERO,
                adjusted_matrix: None,
                post_processing_texture: None,
                present_bind_group: None,
            })),
        });

        if let Some((props, mut rect)) = self.props.get().zip(self.layout_rect.get()) {
            let own_clip = self.clip.get();

            if own_clip.size().x > 0.0
                && own_clip.size().y > 0.0
                && (Some(own_clip) != clip || paint.inner.borrow_mut().repaint)
            {
                paint.inner.borrow_mut().repaint = true;

                rect.set_size(rect.size() * ctx.layout.scale_factor());
                rect.set_pos(rect.pos() * ctx.layout.scale_factor());

                let inside = own_clip.constrain(rect);
                if !inside.size().abs_diff_eq(Vec2::ZERO, 0.1) {
                    let sign =
                        (rect.max() - rect.size() / 2.0) - (inside.max() - inside.size() / 2.0);

                    let sx = rect.size().x / inside.size().x;
                    let sy = rect.size().y / inside.size().y;

                    let dx = (sx - 1.0) * sign.x.signum();
                    let dy = (sy - 1.0) * sign.y.signum();

                    paint.inner.borrow_mut().adjusted_matrix = Some(
                        Matrix4::from_translation(vec3(dx, dy, 0.0))
                            * props
                                .instance
                                .get_world_matrix()
                                .unwrap_or(Matrix4::IDENTITY)
                            * Matrix4::from_scale(vec3(sx, sy, 1.0)),
                    );
                }
            }
        }

        if let Some(clip) = clip {
            self.clip.set(clip);
            paint.inner.borrow_mut().clip = clip;
        }

        if let Some(layer) = ctx.paint.layers_mut().current_mut() {
            layer
                .calls
                .push((PaintCall::Custom(yakui_wgpu::cast(paint.clone())), clip));
        }
    }
}
