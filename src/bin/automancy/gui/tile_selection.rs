use egui::scroll_area::ScrollBarVisibility;
use egui::{vec2, Context, CursorIcon, Margin, ScrollArea, Sense, TopBottomPanel, Ui, Vec2};
use futures::channel::mpsc;

use automancy::tile_entity::TileModifier;
use automancy_defs::cgmath::point3;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_defs::math::{rad, Matrix4};
use automancy_defs::rendering::InstanceData;

use crate::gui::default_frame;
use crate::renderer::GuiInstances;
use crate::setup::GameSetup;

/// Draws the tile selection.
fn draw_tile_selection(
    setup: &GameSetup,
    gui_instances: &mut GuiInstances,
    ui: &mut Ui,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    mut selection_send: mpsc::Sender<Id>,
) {
    let size = ui.available_height();

    setup
        .resource_man
        .ordered_tiles
        .iter()
        .flat_map(|id| {
            setup
                .resource_man
                .registry
                .tile(*id)
                .unwrap()
                .models
                .get(*selected_tile_modifiers.get(id).unwrap_or(&0) as usize)
                .map(|id| setup.resource_man.get_model(*id))
                .map(|model| (*id, model))
        })
        .for_each(|(id, model)| {
            let (ui_id, rect) = ui.allocate_space(vec2(size, size));
            let response = ui.interact(rect, ui_id, Sense::click());

            response
                .clone()
                .on_hover_text(setup.resource_man.tile_name(&id));
            response.clone().on_hover_cursor(CursorIcon::Grab);

            let hover = if response.hovered() {
                ui.ctx()
                    .animate_value_with_time(ui.next_auto_id(), 0.7, 0.3)
            } else {
                ui.ctx()
                    .animate_value_with_time(ui.next_auto_id(), 0.2, 0.3)
            };
            if response.clicked() {
                selection_send.try_send(id).unwrap();
            }

            let matrix = Matrix4::from_angle_x(rad(hover));

            gui_instances.push((
                InstanceData::default()
                    .with_model_matrix(matrix)
                    .with_light_pos(point3(0.0, 1.0, 8.0)),
                model,
                (
                    Some(rect),
                    Some(ui.clip_rect().shrink2(Vec2::new(2.0, 0.0))),
                ),
            ));
        });
}

/// Creates the tile selection GUI.
pub fn tile_selections(
    setup: &GameSetup,
    gui_instances: &mut GuiInstances,
    context: &Context,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    selection_send: mpsc::Sender<Id>,
) {
    TopBottomPanel::bottom("tile_selections")
        .show_separator_line(false)
        .resizable(false)
        .frame(default_frame().outer_margin(Margin::same(10.0)))
        .show(context, |ui| {
            ui.spacing_mut().scroll_bar_outer_margin = 0.0;

            ScrollArea::horizontal()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_height(80.0);

                        draw_tile_selection(
                            setup,
                            gui_instances,
                            ui,
                            selected_tile_modifiers,
                            selection_send,
                        );
                    });
                });
        });
}
