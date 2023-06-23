use egui::output::OpenUrl;
use egui::{Response, Ui, Widget};

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct HyperlinkWidget<T: Widget> {
    url: String,
    widget: T,
}

impl<T: Widget> HyperlinkWidget<T> {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(widget: T, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            widget,
        }
    }
}

impl<T: Widget> Widget for HyperlinkWidget<T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { url, widget } = self;

        let response = ui.add(widget);
        if response.clicked() {
            let modifiers = ui.ctx().input(|i| i.modifiers);
            ui.ctx().output_mut(|o| {
                o.open_url = Some(OpenUrl {
                    url: url.clone(),
                    new_tab: modifiers.any(),
                });
            });
        }
        if response.middle_clicked() {
            ui.ctx().output_mut(|o| {
                o.open_url = Some(OpenUrl {
                    url: url.clone(),
                    new_tab: true,
                });
            });
        }
        response.on_hover_text(url)
    }
}
