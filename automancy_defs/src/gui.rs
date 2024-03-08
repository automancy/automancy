use egui::epaint::Shadow;
use egui::output::OpenUrl;
use egui::style::{Interaction, Spacing, WidgetVisuals, Widgets};
use egui::FontFamily::{Monospace, Proportional};
use egui::{
    Color32, Context, FontDefinitions, FontId, Margin, Response, Rounding, Stroke, Style,
    TextStyle, Ui, Visuals, Widget,
};
use egui_winit::State;
use flexstr::SharedStr;
use winit::window::Window;

pub struct Gui {
    pub renderer: egui_wgpu::Renderer,
    pub context: Context,
    pub state: State,
    pub fonts: FontDefinitions,
}

pub fn set_font(font: SharedStr, gui: &mut Gui) {
    gui.fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, font.to_string());
    gui.fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, font.to_string());
    gui.context.set_fonts(gui.fonts.clone());
}

/// Initialize the GUI style.
fn init_styles(context: &Context) {
    let light = Visuals::light();
    context.set_style(Style {
        text_styles: [
            (TextStyle::Small, FontId::new(9.0, Proportional)),
            (TextStyle::Body, FontId::new(13.0, Proportional)),
            (TextStyle::Button, FontId::new(13.0, Proportional)),
            (TextStyle::Heading, FontId::new(19.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, Monospace)),
        ]
        .into(),
        visuals: Visuals {
            window_fill: Color32::from_white_alpha(190),
            panel_fill: Color32::from_white_alpha(190),

            window_rounding: Rounding::same(8.0),
            menu_rounding: Rounding::same(0.0),

            window_shadow: Shadow {
                extrusion: 8.0,
                color: Color32::from_black_alpha(64),
            },
            popup_shadow: Shadow {
                extrusion: 4.0,
                color: Color32::from_black_alpha(64),
            },

            window_stroke: Stroke::NONE,

            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(248),
                    bg_fill: Color32::from_gray(170),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(180)), // separators, indentation lines
                    fg_stroke: Stroke::new(1.5, Color32::from_gray(40)),  // normal text color
                    rounding: Rounding::same(1.5),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(210), // button background
                    bg_fill: Color32::from_gray(210),      // checkbox background
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(180)),
                    fg_stroke: Stroke::new(1.5, Color32::from_gray(40)), // button text
                    rounding: Rounding::same(3.0),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(220),
                    bg_stroke: Stroke::new(2.0, Color32::from_gray(180)), // e.g. hover over window edge or button
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 0.0,
                },
                active: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(170),
                    bg_fill: Color32::from_gray(190),
                    bg_stroke: Stroke::new(2.0, Color32::BLACK),
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 0.0,
                },
                open: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(210),
                    bg_stroke: Stroke::new(2.0, Color32::from_gray(160)),
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 0.0,
                },
            },
            slider_trailing_fill: true,
            ..light
        },
        spacing: Spacing {
            window_margin: Margin::same(10.0),
            ..Default::default()
        },
        interaction: Interaction {
            show_tooltips_only_when_still: false,
            tooltip_delay: 0.0,
            ..Default::default()
        },
        ..Default::default()
    });
}

/// Initializes the GUI.
pub fn init_gui(renderer: egui_wgpu::Renderer, window: &Window) -> Gui {
    let context = Context::default();
    egui_extras::install_image_loaders(&context);

    context.tessellation_options_mut(|o| {
        o.coarse_tessellation_culling = false;
        o.feathering = false;
    });
    init_styles(&context);

    let viewport_id = context.viewport_id();

    let gui = Gui {
        context: context.clone(),
        state: State::new(
            context,
            viewport_id,
            window,
            Some(window.scale_factor() as f32),
            None,
        ),
        renderer,
        fonts: FontDefinitions::default(),
    };

    gui
}

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
