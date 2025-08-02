use automancy_ui::custom::{RenderObject, RenderObjectDiscriminants};
use hashbrown::HashMap;
use range_set_blaze::RangeSetBlaze;
use yakui::paint::UserPaintCallId;

use crate::gpu::GuiResources;

pub mod game_object;

#[derive(Debug)]
pub struct UiRenderer {
    pub gui_resources: GuiResources,
    pub objects: HashMap<UserPaintCallId, RenderObject>,
    pub object_id_map: HashMap<RenderObjectDiscriminants, RangeSetBlaze<UserPaintCallId>>,
}

impl UiRenderer {
    pub fn new(gui_resources: GuiResources) -> Self {
        Self {
            gui_resources,
            objects: Default::default(),
            object_id_map: Default::default(),
        }
    }

    pub fn get_objects_of(
        &self,
        ty: RenderObjectDiscriminants,
    ) -> HashMap<UserPaintCallId, RenderObject> {
        let ranges = self.object_id_map.get(ty).unwrap_or_default();

        ranges
            .iter()
            .flat_map(|id| self.objects.get(id).map(|v| (id, v)))
            .collect()
    }

    pub fn start_render(&mut self) {
        if automancy_ui::custom::should_rerender() {
            let objects = automancy_ui::custom::take_objects();

            let mut object_id_map = HashMap::new();
            for (id, object) in &objects {
                let ty = RenderObjectDiscriminants::from(object);

                if !object_id_map.contains_key(&ty) {
                    object_id_map.insert(ty, RangeSetBlaze::new());
                }
                object_id_map.get_mut(&ty).unwrap().insert(*id);
            }

            self.object_id_map = object_id_map;
            self.objects = objects;
        }
    }
}
