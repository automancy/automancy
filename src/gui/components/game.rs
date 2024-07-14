use std::{cell::Cell, iter, time::Instant};

use automancy_defs::{
    glam::vec3,
    id::Id,
    math::Matrix4,
    rendering::{GameUBO, InstanceData, IntermediateUBO, PostProcessingUBO},
};
use crunch::{Item, Rotation};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferUsages, Color,
    Extent3d, IndexFormat, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureDescriptor,
    TextureDimension, TextureUsages, TextureViewDescriptor,
};
use yakui::{
    paint::{CustomPaintCall, PaintCall},
    util::widget,
    widget::Widget,
    Rect, Response, UVec2, Vec2,
};
use yakui_wgpu::CallbackTrait;

use crate::{
    gpu::{
        self, make_fxaa_bind_group, IndirectInstanceDrawData, DEPTH_FORMAT, MODEL_DEPTH_CLEAR,
        MODEL_DEPTH_FORMAT, NORMAL_CLEAR, NORMAL_FORMAT,
    },
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
    world_matrix: Matrix4,
}

pub fn ui_game_object(
    instance: InstanceData,
    model: Id,
    size: Vec2,
    world_matrix: Option<Matrix4>,
) -> Response<()> {
    GameElement::new(instance, model, size, world_matrix).show()
}

impl GameElement {
    pub fn new(
        instance: InstanceData,
        model: Id,
        size: Vec2,
        world_matrix: Option<Matrix4>,
    ) -> Self {
        let index = INDEX_COUNTER.get();

        let result = Self {
            instance,
            model,
            index,
            size,
            world_matrix: world_matrix.unwrap_or(Matrix4::IDENTITY),
        };
        INDEX_COUNTER.set(index + 1);

        result
    }

    #[track_caller]
    pub fn show(self) -> Response<()> {
        widget::<GameElementWidget>(Some(self))
    }
}

#[derive(Debug)]
pub struct GameElementPaint {
    props: GameElement,
    present_uniform: Option<Buffer>,
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
            _packed_size,
            _rects,
            _present_texture,
        ): &mut YakuiRenderResources,
    ) {
        let props = self.props;
        let start_instant = START_INSTANT.get().unwrap();
        try_add_animation(resource_man, start_instant, props.model, animation_map);

        instances.as_mut().unwrap().push((
            props.instance,
            props.world_matrix,
            props.model,
            (props.index, props.size),
        ));
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
            packed_size,
            rects,
            present_texture,
        ): &mut YakuiRenderResources,
    ) {
        if let Some(instances) = instances.take() {
            let items = instances
                .iter()
                .map(|(.., (index, size))| {
                    Item::new(
                        *index,
                        size.x.round() as usize * 2,
                        size.y.round() as usize * 2,
                        Rotation::None,
                    )
                })
                .collect::<Vec<_>>();

            let packed =
                crunch::pack_into_po2(device.limits().max_texture_dimension_2d as usize, items)
                    .expect("gui items exceed max texture size.");

            let IndirectInstanceDrawData {
                opaques,
                non_opaques,
                matrix_data,
                world_matrix_data,
                draw_data,
            } = gpu::indirect_instance(resource_man, instances, animation_map, false);

            if draw_data.opaques.is_empty() && draw_data.non_opaques.is_empty() {
                return;
            }

            let gui_resources = gui_resources.as_mut().unwrap();

            gpu::update_instance_buffer(
                device,
                queue,
                &mut gui_resources.opaques_instance_buffer,
                &opaques,
            );
            gpu::update_instance_buffer(
                device,
                queue,
                &mut gui_resources.non_opaques_instance_buffer,
                &non_opaques,
            );

            queue.write_buffer(
                &gui_resources.matrix_data_buffer,
                0,
                bytemuck::cast_slice(matrix_data.into_iter().collect::<Vec<_>>().as_slice()),
            );

            queue.write_buffer(
                &gui_resources.world_matrix_data_buffer,
                0,
                bytemuck::cast_slice(world_matrix_data.into_iter().collect::<Vec<_>>().as_slice()),
            );

            queue.write_buffer(
                &gui_resources.uniform_buffer,
                0,
                bytemuck::cast_slice(&[GameUBO::default()]),
            );

            let size = UVec2::new(packed.w as u32, packed.h as u32);
            *packed_size = Some(size);

            rects.clear();
            for item in packed.items.iter() {
                if item.data >= rects.len() {
                    rects.extend(iter::repeat(None).take(item.data - rects.len() + 1));
                }

                rects[item.data] = Some(item.rect);
            }

            let color_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
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
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let model_depth_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: MODEL_DEPTH_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let normal_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
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
                    flags: 0,
                    ..Default::default()
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
            let normal = normal_texture.create_view(&TextureViewDescriptor::default());
            let model_depth = model_depth_texture.create_view(&TextureViewDescriptor::default());

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
                            resource: BindingResource::TextureView(&model_depth),
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

            let post_processing_texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: *surface_format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let antialiasing_bind_group = make_fxaa_bind_group(
                device,
                &global_resources.fxaa_bind_group_layout,
                &post_processing_texture.create_view(&TextureViewDescriptor::default()),
                &global_resources.filtering_sampler,
            );

            *present_texture = Some(device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: size.x * 3 / 2,
                    height: size.y * 3 / 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: *surface_format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));

            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Render Pass"),
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
                            view: &model_depth,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(MODEL_DEPTH_CLEAR),
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

                {
                    render_pass.set_pipeline(&global_resources.game_pipeline);
                    render_pass.set_bind_group(0, &gui_resources.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, global_resources.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        global_resources.index_buffer.slice(..),
                        IndexFormat::Uint16,
                    );

                    render_pass
                        .set_vertex_buffer(1, gui_resources.opaques_instance_buffer.slice(..));
                    for (draw, (rect_index, ..)) in draw_data.opaques.into_iter() {
                        if let Some(rect) = rects[rect_index] {
                            render_pass.set_viewport(
                                rect.x as f32,
                                rect.y as f32,
                                rect.w as f32,
                                rect.h as f32,
                                0.0,
                                1.0,
                            );

                            render_pass.draw_indexed(
                                draw.first_index..(draw.first_index + draw.index_count),
                                draw.base_vertex,
                                draw.first_instance..(draw.first_instance + draw.instance_count),
                            );
                        }
                    }

                    render_pass
                        .set_vertex_buffer(1, gui_resources.non_opaques_instance_buffer.slice(..));
                    for (draw, (rect_index, ..)) in draw_data.non_opaques.into_iter() {
                        if let Some(rect) = rects[rect_index] {
                            render_pass.set_viewport(
                                rect.x as f32,
                                rect.y as f32,
                                rect.w as f32,
                                rect.h as f32,
                                0.0,
                                1.0,
                            );

                            render_pass.draw_indexed(
                                draw.first_index..(draw.first_index + draw.index_count),
                                draw.base_vertex,
                                draw.first_instance..(draw.first_instance + draw.instance_count),
                            );
                        }
                    }
                }
            }

            {
                let view = post_processing_texture.create_view(&TextureViewDescriptor::default());

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Post Processing Render Pass"),
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

            {
                let view = present_texture
                    .as_ref()
                    .unwrap()
                    .create_view(&TextureViewDescriptor::default());

                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("UI Model Antialiasing Render Pass"),
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

                render_pass.set_pipeline(&global_resources.fxaa_pipeline);
                render_pass.set_bind_group(0, &antialiasing_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        if self.present_uniform.is_none() {
            self.present_uniform = Some(device.create_buffer_init(&BufferInitDescriptor {
                label: Some("UI Model Present Uniform Buffer"),
                contents: bytemuck::cast_slice(&[IntermediateUBO {
                    viewport_size: [0.0; 2],
                    viewport_pos: [0.0; 2],
                }]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }));
        }

        if let Some((Some(rect), packed_size)) =
            rects.get(self.props.index).cloned().zip(*packed_size)
        {
            queue.write_buffer(
                self.present_uniform.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&[IntermediateUBO {
                    viewport_size: [
                        rect.w as f32 / packed_size.x as f32,
                        rect.h as f32 / packed_size.y as f32,
                    ],
                    viewport_pos: [
                        rect.x as f32 / packed_size.x as f32,
                        rect.y as f32 / packed_size.y as f32,
                    ],
                }]),
            );
        }

        self.present_bind_group = Some(
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &global_resources.intermediate_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &present_texture
                                .as_ref()
                                .unwrap()
                                .create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&global_resources.nonfiltering_sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: self.present_uniform.as_ref().unwrap().as_entire_binding(),
                    },
                ],
            }),
        );
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
            _packed_size,
            _rects,
            _present_texture,
        ): &'a YakuiRenderResources,
    ) {
        if let Some(present_bind_group) = self.present_bind_group.as_ref() {
            render_pass.set_pipeline(&global_resources.multisampled_present_pipeline);
            render_pass.set_bind_group(0, present_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameElementWidget {
    props: Cell<Option<GameElement>>,
    clip: Cell<Rect>,
    adjusted_matrix: Cell<Option<Matrix4>>,
}

impl Widget for GameElementWidget {
    type Props<'a> = Option<GameElement>;
    type Response = ();

    fn new() -> Self {
        Self {
            props: Cell::default(),
            clip: Cell::new(Rect::ZERO),
            adjusted_matrix: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props.set(props);
    }

    fn layout(
        &self,
        _ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> yakui::Vec2 {
        if let Some(paint) = self.props.get() {
            constraints.constrain(paint.size)
        } else {
            constraints.min
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let paint_clip = ctx.paint.get_current_clip();

        if let Some(clip) = paint_clip {
            self.clip.set(clip);
        }

        let mut new_clip_rect = Rect::ZERO;
        if let Some(mut rect) = ctx.layout.get(ctx.dom.current()).map(|v| v.rect) {
            let clip = ctx.layout.unscaled_viewport().constrain(self.clip.get());

            if clip.size().x > 0.0 && clip.size().y > 0.0 {
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
                            * Matrix4::from_scale(vec3(sx, sy, 1.0))
                            * self.props.get().unwrap().world_matrix,
                    ));
                }
                new_clip_rect = inside;
            }
        }

        if let Some(layer) = ctx.paint.layers_mut().current_mut() {
            let mut props = self.props.get().unwrap();
            if let Some(matrix) = self.adjusted_matrix.get() {
                props.world_matrix = matrix;
            }

            let paint = Box::new(GameElementPaint {
                props,
                present_bind_group: None,
                present_uniform: None,
            });

            layer.calls.push((
                PaintCall::Custom(CustomPaintCall { callback: paint }),
                Some(new_clip_rect),
            ));
        }
    }
}
