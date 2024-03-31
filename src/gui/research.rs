use egui::{vec2, ScrollArea, Sense, Window};

use automancy_defs::glam::vec3;
use automancy_defs::graph::visit::Topo;
use automancy_defs::rendering::InstanceData;

use crate::gui::{GameEguiCallback, MEDIUM_ICON_SIZE};
use crate::GameState;

pub fn research_ui(state: &mut GameState) {
    let mut open = true;

    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.research_menu_title]
            .as_str(),
    )
    .resizable(true)
    .open(&mut open)
    .max_size(state.gui.context.screen_rect().size())
    .show(&state.gui.context.clone(), |ui| {
        ScrollArea::vertical().max_width(240.0).show(ui, |ui| {
            let mut visitor = Topo::new(&state.resource_man.registry.researches);
            while let Some(idx) = visitor.next(&state.resource_man.registry.researches) {
                let research = &state.resource_man.registry.researches[idx];

                if ui
                    .group(|ui| {
                        let icon = research.icon;
                        let icon = state.resource_man.get_model(icon);

                        let (rect, _icon_response) = ui.allocate_exact_size(
                            vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                            Sense::click(),
                        );

                        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                            rect,
                            GameEguiCallback::new(
                                InstanceData::default()
                                    .with_model_matrix(research.icon_mode.model_matrix())
                                    .with_world_matrix(research.icon_mode.world_matrix())
                                    .with_light_pos(vec3(0.0, 4.0, 14.0), None),
                                icon,
                                rect,
                                ui.ctx().screen_rect(),
                            ),
                        ));

                        ui.label(state.resource_man.translates.research[&research.name].as_str())
                            .on_hover_text(
                                state.resource_man.translates.research[&research.description]
                                    .as_str(),
                            );
                    })
                    .response
                    .clicked()
                {
                    state.gui_state.selected_research = Some(research.id);
                };
            }
        });
    });

    if !open {
        state.gui_state.research_open = false;
    }
}
