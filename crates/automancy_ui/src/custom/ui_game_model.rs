use automancy_data::rendering::draw::GameUniformData;
use automancy_game::scripting::render::RenderCommand;
use automancy_rendering::{GameInstanceId, GameInstanceManager, ModelManager};

use crate::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct UiGameInstanceIndex {
    paint_id: UserPaintCallId,
    render_id: RenderId,
    __p: u32,
}
type UiGameInstanceIdMap = BTreeMap<UserPaintCallId, BTreeSet<(ModelId, UiGameInstanceIndex)>>;

fn ui_instance_manager() -> GameInstanceManager<UiGameInstanceIndex> {
    GameInstanceManager::new("UI_RENDER")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UiGameModelObject {
    pub model: GenericModel,
    pub instance: GameDrawInstance,
}

#[derive(Debug)]
struct UiRenderResources<T> {
    size: UVec2,
    format: Option<wgpu::TextureFormat>,
    samples: u32,

    data: T,
}

#[derive(Debug)]
struct UiModelRenderResources {
    game_res: gpu::GameRenderResources,
    compose_pipeline: gpu::pipeline::ComposePipeline,
}

#[derive(Debug, Default)]
pub struct UiGameModelRenderer {
    instance_man: BTreeMap<yakui::TextureId, GameInstanceManager<UiGameInstanceIndex>>,
    instance_man_with_picking_matrix: BTreeMap<yakui::TextureId, GameInstanceManager<UiGameInstanceIndex>>,
    instance_id_map: BTreeMap<yakui::TextureId, UiGameInstanceIdMap>,

    modified: BTreeMap<yakui::TextureId, bool>,

    resources: BTreeMap<yakui::TextureId, UiRenderResources<UiModelRenderResources>>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl UiGameModelRenderer {
    #[inline]
    pub fn remove(&mut self, paint_id: UserPaintCallId, texture_id: yakui::TextureId) {
        if let Some(ids) = self.instance_id_map.entry(texture_id).or_default().remove(&(paint_id)) {
            let instance_man = self.instance_man.entry(texture_id).or_insert_with(ui_instance_manager);

            for (model_id, index) in ids {
                let id = GameInstanceId {
                    model_id,
                    index,
                };

                instance_man.remove(id);
            }

            self.modified.insert(texture_id, true);
        }
    }

    #[inline]
    pub fn modify(
        &mut self,
        model_man: &ModelManager,
        instance: GameDrawInstance,
        paint_id: UserPaintCallId,
        texture_id: yakui::TextureId,
    ) {
        self.modified.insert(texture_id, true);

        if let Some(instance_ids) = self.instance_id_map.entry(texture_id).or_default().get(&paint_id) {
            let instance_man = self.instance_man.entry(texture_id).or_insert_with(ui_instance_manager);

            for &(model_id, index) in instance_ids {
                let id = GameInstanceId {
                    model_id,
                    index,
                };

                instance_man.set_matrix(model_man, id, (Some(instance.model_matrix), Some(instance.world_matrix)));
                instance_man.modify_instances(model_man, id, |_, v| {
                    v.color_offset = instance.color_offset;
                });
            }
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        resource_man: &ResourceManager,
        model_man: &ModelManager,
        object: UiGameModelObject,
        paint_id: UserPaintCallId,
        texture_id: yakui::TextureId,
    ) {
        #[allow(clippy::too_many_arguments)]
        #[inline]
        fn insert(
            instance_man: &mut GameInstanceManager<UiGameInstanceIndex>,
            instance_id_map: &mut UiGameInstanceIdMap,
            model_man: &ModelManager,
            instance: GameDrawInstance,
            id @ GameInstanceId {
                model_id,
                index,
            }: GameInstanceId<UiGameInstanceIndex>,
        ) {
            instance_man.insert(model_man, id, instance);

            instance_id_map.entry(index.paint_id).or_default().insert((model_id, index));
        }

        let instance_man = self.instance_man.entry(texture_id).or_insert_with(ui_instance_manager);
        let instance_id_map = self.instance_id_map.entry(texture_id).or_default();

        match object.model {
            GenericModel::None => {},
            GenericModel::Tile(id) => {
                let commands = if !id.is_none() {
                    let Some(commands) = tile_entity::collect_render_commands(
                        resource_man,
                        id,
                        TileCoord::ZERO,
                        &mut DataMap::new(),
                        &mut Default::default(),
                        true,
                        false,
                    ) else {
                        return;
                    };

                    commands
                } else {
                    automancy_game::scripting::render::util::track_none(resource_man, TileCoord::ZERO).to_vec()
                };

                for command in commands {
                    match command {
                        RenderCommand::Track {
                            render_id,
                            model_id,
                        } => {
                            insert(
                                instance_man,
                                instance_id_map,
                                model_man,
                                object.instance,
                                GameInstanceId {
                                    model_id,
                                    index: UiGameInstanceIndex {
                                        paint_id,
                                        render_id,
                                        __p: 0,
                                    },
                                },
                            );
                        },
                        RenderCommand::Transform {
                            render_id,
                            model_id,
                            model_matrix,
                        } => {
                            instance_man.mul_matrix_right(
                                model_man,
                                GameInstanceId {
                                    model_id,
                                    index: UiGameInstanceIndex {
                                        paint_id,
                                        render_id,
                                        __p: 0,
                                    },
                                },
                                (Some(model_matrix), None),
                            );
                        },
                        RenderCommand::Untrack {
                            render_id,
                            model_id,
                        } => {
                            let index = UiGameInstanceIndex {
                                paint_id,
                                render_id,
                                __p: 0,
                            };

                            instance_man.remove(GameInstanceId {
                                model_id,
                                index,
                            });

                            instance_id_map.get_mut(&paint_id).unwrap().remove(&(model_id, index));
                        },
                    }
                }
            },
            GenericModel::Item(item_id) => {
                let render_id = RenderId::none();
                let model_id = resource_man.item_model_or_missing(item_id);

                insert(
                    instance_man,
                    instance_id_map,
                    model_man,
                    object.instance,
                    GameInstanceId {
                        model_id,
                        index: UiGameInstanceIndex {
                            paint_id,
                            render_id,
                            __p: 0,
                        },
                    },
                );
            },
            GenericModel::Plain(model_id) => {
                let render_id = RenderId::none();

                insert(
                    instance_man,
                    instance_id_map,
                    model_man,
                    object.instance,
                    GameInstanceId {
                        model_id,
                        index: UiGameInstanceIndex {
                            paint_id,
                            render_id,
                            __p: 0,
                        },
                    },
                );
            },
        }
    }

    #[inline]
    pub fn flush_removal(&mut self, model_man: &ModelManager, texture_id: yakui::TextureId) {
        let Some(instance_man) = self.instance_man.get_mut(&texture_id) else {
            return;
        };

        instance_man.flush_removal(model_man);
    }

    #[inline]
    pub fn update_pipelines(&mut self, res: &gpu::RenderResources, atlas_man: &custom::AtlasManager, surface: &yakui_wgpu::SurfaceInfo) {
        for atlas in atlas_man.atlases() {
            if atlas.texture_size.x == 0 || atlas.texture_size.y == 0 {
                continue;
            }

            let ui_res = self.resources.get(&atlas.texture_id);
            if ui_res.is_none()
                || ui_res.is_some_and(|res| {
                    res.size != atlas.texture_size || res.format != Some(surface.format) || res.samples != surface.sample_count
                })
            {
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    width: atlas.texture_size.x,
                    height: atlas.texture_size.y,
                    // these shouldn't matter
                    present_mode: wgpu::PresentMode::default(),
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::default(),
                    view_formats: vec![],
                };

                let game_res = gpu::GameRenderResources::new(
                    &res.device,
                    &config,
                    &res.global_res,
                    options::AAType::None,
                    ui_res.map(|v| &v.data.game_res),
                );

                let compose_pipeline = gpu::pipeline::ComposePipeline::new(
                    &res.device,
                    &config,
                    &res.global_res,
                    gpu::pipeline::ComposePipelineArgs {
                        sampler: &res.global_res.point_sampler,
                        textures: &[&game_res.render_textures.output_texture],
                    },
                );

                self.resources.insert(
                    atlas.texture_id,
                    UiRenderResources {
                        size: atlas.texture_size,
                        format: Some(surface.format),
                        samples: surface.sample_count,

                        data: UiModelRenderResources {
                            game_res,
                            compose_pipeline,
                        },
                    },
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        global_res: &gpu::GlobalResources,
        encoder: &mut wgpu::CommandEncoder,
        render_state: &mut renderer::AutomancyRenderState,
        atlas_man: &custom::AtlasManager,
        frame_start: Instant,
    ) {
        for atlas in atlas_man.atlases() {
            if atlas.texture_size.x == 0 || atlas.texture_size.y == 0 {
                continue;
            }

            let Some(instance_id_map) = self.instance_id_map.get_mut(&atlas.texture_id) else {
                continue;
            };

            {
                let Some(instance_man) = self.instance_man.get_mut(&atlas.texture_id) else {
                    continue;
                };

                let modified = self.modified.entry(atlas.texture_id).or_insert(true);
                if *modified {
                    let mut cloned_instance_man = instance_man.clone();
                    // mark uploaded so we don't keep uploading the cloned one unnecessarily
                    instance_man.mark_uploaded();

                    for (&paint_id, ids) in instance_id_map.iter() {
                        let atlas_entry = atlas_man.get_atlas_entry(paint_id);
                        debug_assert_eq!(atlas.texture_id, atlas_entry.texture_id);

                        for &(model_id, index) in ids {
                            let rect = atlas_entry.rect;

                            let scale = math::Matrix4::scaling_3d(rect.size().unyak().with_z(1.0));
                            let translation = math::Matrix4::translation_3d(
                                (((rect.center() * 2.0).unyak() - 1.0) * math::Vec2::new(1.0, -1.0)).with_z(0.0),
                            );

                            cloned_instance_man.mul_matrix_left(
                                &render_state.model_man,
                                GameInstanceId {
                                    model_id,
                                    index,
                                },
                                (None, Some(translation * scale)),
                            );
                        }
                    }
                    self.instance_man_with_picking_matrix.insert(atlas.texture_id, cloned_instance_man);

                    *modified = false;
                }
            }

            let instance_man = self.instance_man_with_picking_matrix.get_mut(&atlas.texture_id).unwrap();
            {
                let ui_res = self.resources.get_mut(&atlas.texture_id).unwrap();

                instance_man.pre_render(
                    device,
                    queue,
                    global_res,
                    &mut ui_res.data.game_res,
                    &mut render_state.model_man,
                    frame_start,
                );

                instance_man
                    .animations
                    .upload(queue, &ui_res.data.game_res.game_pipeline.animation_matrix_buffer);

                let draw_calls = instance_man.collect_draw_calls(device, queue, &mut ui_res.data.game_res);

                let draws_counts = [draw_calls.opaque.buffer.len() as u32, draw_calls.non_opaque.buffer.len() as u32];
                let game_uniform = GameUniformData {
                    camera_pos: math::Vec3::new(0.0, -0.1, 1.0),
                    camera_bounds: math::Rect::from_pos_size(math::Vec2::zero(), math::Vec2::one()),
                    ..Default::default()
                };

                ui_res.data.game_res.render(
                    queue,
                    global_res,
                    encoder,
                    draws_counts,
                    (
                        gpu::data::GpuGameUniformData::new(&game_uniform),
                        gpu::data::GpuGameLightingUniformData::new(&game_uniform),
                        gpu::data::GpuPostProcessingUniformData {
                            flags: gpu::data::GpuPostProcessingUniformData::FLAG_NONE,
                            ..Default::default()
                        },
                    ),
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Ui Model Present Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &atlas.texture_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });

                    render_pass.set_pipeline(&ui_res.data.compose_pipeline.render_pipeline);
                    render_pass.set_bind_group(0, &ui_res.data.compose_pipeline.textures_bind_group, &[]);
                    render_pass.set_bind_group(1, &ui_res.data.compose_pipeline.uniform_bind_group, &[]);
                    render_pass.draw(0..3, 0..1);
                }
            }
        }
    }

    pub fn move_texture_ids(&mut self, model_man: &ModelManager, changes: &[(yakui::TextureId, yakui::TextureId)]) {
        for &(old_id, new_id) in changes {
            {
                let old = self.instance_man.get_mut(&old_id).unwrap();
                let old_instances = old.to_game_draw_instances(model_man);
                old.clear();

                let new = self.instance_man.entry(new_id).or_insert_with(ui_instance_manager);
                new.flush_removal(model_man);

                for (id, instance) in old_instances {
                    new.insert(model_man, id, instance);
                }
            }

            {
                // TODO use unsafe to reuse allocation here
                let old = std::mem::take(self.instance_id_map.get_mut(&old_id).unwrap());

                let new = self.instance_id_map.entry(new_id).or_default();
                for (id, mut ids) in old {
                    new.entry(id).or_default().append(&mut ids);
                }
            }

            self.modified.insert(old_id, true);
            self.modified.insert(new_id, true);
        }
    }
}
