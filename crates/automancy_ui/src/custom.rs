use std::collections::BTreeMap;

use automancy_rendering::{ModelManager, gpu::RenderResources};
use yakui::{Rect, UVec2, Vec2, paint::UserPaintCallId};
use yakui_wgpu::YakuiWgpu;

use crate::GameModel;

mod ui_game_model;
pub(crate) use ui_game_model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RenderObjectType {
    GameModel(ui_game_model::UiGameModelObject),
}

impl From<GameModel> for RenderObjectType {
    fn from(value: GameModel) -> Self {
        RenderObjectType::GameModel(ui_game_model::UiGameModelObject {
            model: value.model,
            instance: value.instance,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderObject {
    pub ty: RenderObjectType,
    pub size: Vec2,
    pub texture_id: yakui::TextureId,
}

#[derive(Debug)]
pub(crate) struct RenderObjectChange {
    pub paint_id: UserPaintCallId,
    pub old: Option<RenderObject>,
    pub new: Option<RenderObject>,
}

#[derive(Debug, Default)]
pub(crate) struct CustomRenderer {
    pub(crate) game_model_renderer: UiGameModelRenderer,
    pub(crate) atlas_man: AtlasManager,

    prev_objects: Vec<RenderObject>,
    objects: Vec<RenderObject>,
}

#[derive(Debug, Clone)]
pub struct AtlasInfo {
    pub texture_id: yakui::TextureId,
    pub texture_view: wgpu::TextureView,
    pub texture_size: UVec2,
}

#[derive(Debug, Clone)]
pub struct AtlasEntry {
    pub rect: Rect,
    pub texture_id: yakui::TextureId,
}

#[derive(Debug)]
pub(crate) struct AtlasManager {
    pub(crate) unpacked_size: UVec2,
    pub(crate) unpacked_size_inv: Vec2,
    unpacked_rects: BTreeMap<UserPaintCallId, Rect>,
    pub(crate) packed_size: UVec2,
    pub(crate) packed_size_inv: Vec2,
    packed_rects: BTreeMap<UserPaintCallId, Rect>,

    // TODO vec of textures
    unpacked_texture: Option<wgpu::TextureView>,
    unpacked_texture_id: Option<yakui::TextureId>,
    unpacked_texture_size: UVec2,
    packed_texture: Option<wgpu::TextureView>,
    packed_texture_id: Option<yakui::TextureId>,
    packed_texture_size: UVec2,
}

impl Default for AtlasManager {
    fn default() -> Self {
        const UNPACKED_SIZE: UVec2 = UVec2::splat(2048);

        Self {
            unpacked_size: UNPACKED_SIZE,
            unpacked_size_inv: 1.0 / UNPACKED_SIZE.as_vec2(),
            unpacked_rects: Default::default(),
            packed_size: UVec2::ONE,
            packed_size_inv: Vec2::ONE,
            packed_rects: Default::default(),

            unpacked_texture: None,
            unpacked_texture_id: None,
            unpacked_texture_size: UVec2::ZERO,
            packed_texture: None,
            packed_texture_id: None,
            packed_texture_size: UVec2::ZERO,
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl AtlasManager {
    pub fn atlases(&self) -> impl Iterator<Item = AtlasInfo> {
        [
            Some(AtlasInfo {
                texture_id: self.unpacked_texture_id.unwrap(),
                texture_view: self.unpacked_texture.clone().unwrap(),
                texture_size: self.unpacked_size,
            }),
            self.packed_texture_id.map(|texture_id| AtlasInfo {
                texture_id,
                texture_view: self.packed_texture.clone().unwrap(),
                texture_size: self.packed_size,
            }),
        ]
        .into_iter()
        .flatten()
    }

    pub fn get_atlas_entry(&self, id: UserPaintCallId) -> AtlasEntry {
        if let Some(&rect) = self.packed_rects.get(&id) {
            return AtlasEntry {
                rect: rect * self.packed_size_inv,
                texture_id: self.packed_texture_id.unwrap(),
                //texture: self.packed_texture.clone().unwrap(),
            };
        }

        let &rect = self.unpacked_rects.get(&id).unwrap();
        AtlasEntry {
            rect: rect * self.unpacked_size_inv,
            texture_id: self.unpacked_texture_id.unwrap(),
            //texture: self.unpacked_texture.clone().unwrap(),
        }
    }

    pub fn update_textures(&mut self, yakui_wgpu: &mut YakuiWgpu, res: &mut RenderResources) {
        if self.unpacked_size.x != 0
            && self.unpacked_size.y != 0
            && (self.unpacked_texture.is_none() || self.unpacked_texture_size != self.unpacked_size)
        {
            let texture = res.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("yakui CustomRenderer Unpacked Atlas Texture"),
                size: wgpu::Extent3d {
                    width: self.unpacked_size.x,
                    height: self.unpacked_size.y,
                    ..Default::default()
                },
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                view_formats: &[],
            });
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            if let Some(id) = self.unpacked_texture_id {
                yakui_wgpu.update_texture(id, texture_view.clone());
            } else {
                self.unpacked_texture_id = Some(yakui_wgpu.add_texture(
                    texture_view.clone(),
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Nearest,
                    wgpu::MipmapFilterMode::Nearest,
                    wgpu::AddressMode::ClampToEdge,
                ));
            }
            self.unpacked_texture_size = self.unpacked_size;
            self.unpacked_texture = Some(texture_view);
        }

        if self.packed_size.x != 0
            && self.packed_size.y != 0
            && (self.packed_texture.is_none() || self.packed_texture_size != self.packed_size)
        {
            let texture = res.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("yakui CustomRenderer Packed Atlas Texture"),
                size: wgpu::Extent3d {
                    width: self.packed_size.x,
                    height: self.packed_size.y,
                    ..Default::default()
                },
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                view_formats: &[],
            });
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            if let Some(id) = self.packed_texture_id {
                yakui_wgpu.update_texture(id, texture_view.clone());
            } else {
                self.packed_texture_id = Some(yakui_wgpu.add_texture(
                    texture_view.clone(),
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Nearest,
                    wgpu::MipmapFilterMode::Nearest,
                    wgpu::AddressMode::ClampToEdge,
                ));
            }
            self.packed_texture_size = self.packed_size;
            self.packed_texture = Some(texture_view);
        }
    }

    fn pack(&mut self, res: &RenderResources, objects: &mut [RenderObject]) -> Vec<(yakui::TextureId, yakui::TextureId)> {
        if !self.unpacked_rects.is_empty() {
            let len = objects.len() as u64;

            // TODO crashes if no space
            let packed_items = crunch::pack_into_po2(
                res.device.limits().max_texture_dimension_2d as usize,
                self.packed_rects
                    .iter()
                    .chain(self.unpacked_rects.iter())
                    .filter(|(id, _)| **id < len)
                    .map(|(id, rect)| crunch::Item {
                        data: *id,
                        w: rect.size().x as usize,
                        h: rect.size().y as usize,
                        rot: crunch::Rotation::None,
                    }),
            )
            .unwrap();

            self.unpacked_rects.clear();

            self.packed_size = UVec2::new(packed_items.w as u32, packed_items.h as u32);
            self.packed_size_inv = 1.0 / self.packed_size.as_vec2();

            self.packed_rects = packed_items
                .items
                .into_iter()
                .map(|item| {
                    (
                        item.data,
                        Rect::from_pos_size(
                            Vec2::new(item.rect.x as f32, item.rect.y as f32),
                            Vec2::new(item.rect.w as f32, item.rect.h as f32),
                        ),
                    )
                })
                .collect();

            for object in objects.iter_mut() {
                object.texture_id = self.unpacked_texture_id.unwrap();
            }

            for &key in self.packed_rects.keys() {
                if let Some(object) = objects.get_mut(key as usize) {
                    object.texture_id = self.packed_texture_id.unwrap();
                } else {
                    log::error!("fklsdjfkljsdlds {key}");
                }
            }

            vec![(self.unpacked_texture_id.unwrap(), self.packed_texture_id.unwrap())]
        } else {
            vec![]
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl CustomRenderer {
    pub fn finish(&mut self, res: &RenderResources, model_man: &ModelManager) {
        let texture_id_changes = self.atlas_man.pack(res, &mut self.objects);
        self.game_model_renderer.move_texture_ids(model_man, &texture_id_changes);

        // reuse allocation
        self.prev_objects.clear();
        self.prev_objects.append(&mut self.objects);
    }

    pub fn build(&mut self) -> Vec<RenderObjectChange> {
        if self.objects == self.prev_objects {
            return vec![];
        }

        let mut changes = vec![];

        for id in self.prev_objects.len()..self.objects.len() {
            changes.push(RenderObjectChange {
                paint_id: id as UserPaintCallId,
                old: None,
                new: self.objects.get(id).copied(),
            });
        }

        for (id, &object) in self.prev_objects.iter().enumerate() {
            if self.objects.get(id) != Some(&object) {
                changes.push(RenderObjectChange {
                    paint_id: id as UserPaintCallId,
                    old: Some(object),
                    new: self.objects.get(id).copied(),
                });
            }
        }

        changes
    }

    pub fn add(&mut self, size: yakui::Vec2, ty: RenderObjectType) -> (yakui::TextureId, Rect) {
        // we haven't pushed the object in yet. index should be last_index + 1, but because we haven't pushed it yet, it's equivalent to .len().
        let index = self.objects.len();
        let id = index as UserPaintCallId;

        // object changed, remove and reset metadata
        if let Some(old) = self.prev_objects.get(index) {
            if old.size == size || old.ty == ty {
                let texture_id = old.texture_id;

                self.objects.push(RenderObject {
                    ty,
                    size,
                    texture_id,
                });
                let rect = self.atlas_man.packed_rects[&id];

                return (texture_id, rect * self.atlas_man.packed_size_inv);
            } else {
                self.atlas_man.packed_rects.remove(&id);
            }
        }

        let pos = if self.atlas_man.unpacked_rects.is_empty() {
            Vec2::ZERO
        } else {
            let mut candidates = self
                .atlas_man
                .unpacked_rects
                .values()
                .filter(|rect| {
                    let max = (rect.max() + size).ceil().as_uvec2();

                    max.x < self.atlas_man.unpacked_size.x && max.y < self.atlas_man.unpacked_size.y
                })
                .map(|rect| rect.pos())
                .collect::<Vec<_>>();
            candidates.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
            candidates.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

            // TODO crashes if no space
            *candidates.first().unwrap()
        };
        let rect = Rect::from_pos_size(pos, size);
        let texture_id = self.atlas_man.unpacked_texture_id.unwrap();
        self.objects.push(RenderObject {
            ty,
            size,
            texture_id,
        });
        self.atlas_man.unpacked_rects.insert(id, rect);

        (texture_id, rect * self.atlas_man.unpacked_size_inv)
    }
}
