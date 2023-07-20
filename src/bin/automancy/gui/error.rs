use egui::vec2;
use egui::{Align, Align2, Window};

use automancy_defs::gui::Gui;
use automancy_resources::error::{error_to_key, error_to_string};

use crate::gui::default_frame;
use crate::setup::GameSetup;

/// Draws an error popup. Can only be called when there are errors in the queue!
pub fn error_popup(setup: &mut GameSetup, gui: &mut Gui) {
    if let Some(error) = setup.resource_man.error_man.peek() {
        Window::new(
            setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.error_popup]
                .to_string(),
        )
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .resizable(false)
        .default_width(300.0)
        .frame(default_frame())
        .show(&gui.context, |ui| {
            ui.label(format!("ID: {}", error_to_key(&error, &setup.resource_man)));
            ui.label(error_to_string(&error, &setup.resource_man));
            //FIXME why are the buttons not right aligned
            ui.with_layout(ui.layout().with_main_align(Align::RIGHT), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            setup.resource_man.translates.gui
                                [&setup.resource_man.registry.gui_ids.btn_confirm]
                                .to_string(),
                        )
                        .clicked()
                    {
                        setup.resource_man.error_man.pop();
                    }
                });
            });
        });
    }
}
