//! Farm's look: cream paper cards over the ant farm by day, charcoal ones by night, inked in the
//! frame's own wood brown, with moss green for whatever is chosen. Text keeps to a readable size
//! whatever the window does.

use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle};

pub struct Ink {
    pub paper: Color32,
    pub card: Color32,
    pub text: Color32,
    pub faint: Color32,
    pub chosen: Color32,
    pub line: Color32,
    pub warn: Color32,
    pub block: Color32,
    pub stage: Color32,
    pub stage_edge: Color32,
}

pub fn ink(dark: bool) -> Ink {
    if dark {
        Ink {
            paper: Color32::from_rgba_unmultiplied(0x2b, 0x27, 0x24, 244),
            card: Color32::from_rgb(0x38, 0x32, 0x2d),
            text: Color32::from_rgb(0xee, 0xe4, 0xd2),
            faint: Color32::from_rgb(0xb3, 0xa6, 0x93),
            chosen: Color32::from_rgb(0x7f, 0xa8, 0x6a),
            line: Color32::from_rgb(0x5a, 0x4c, 0x40),
            warn: Color32::from_rgb(0xe7, 0xb8, 0x5c),
            block: Color32::from_rgb(0xe8, 0x86, 0x7c),
            stage: Color32::from_rgb(0x3b, 0x36, 0x40),
            stage_edge: Color32::from_rgb(0x55, 0x4b, 0x44),
        }
    } else {
        Ink {
            paper: Color32::from_rgba_unmultiplied(0xf6, 0xef, 0xdf, 246),
            card: Color32::from_rgb(0xfb, 0xf7, 0xec),
            text: Color32::from_rgb(0x3e, 0x2c, 0x22),
            faint: Color32::from_rgb(0x8a, 0x73, 0x60),
            chosen: Color32::from_rgb(0x5c, 0x86, 0x48),
            line: Color32::from_rgb(0xc9, 0xb3, 0x92),
            warn: Color32::from_rgb(0xa8, 0x6a, 0x10),
            block: Color32::from_rgb(0xb0, 0x3a, 0x30),
            stage: Color32::from_rgb(0xf3, 0xec, 0xdc),
            stage_edge: Color32::from_rgb(0xb8, 0x96, 0x6e),
        }
    }
}

/// Set Farm's style on both themes, at `text_scale` (1 to 1.5).
pub fn apply(ctx: &egui::Context, text_scale: f32) {
    for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
        let ink = ink(dark);
        ctx.style_mut_of(theme, |style| {
            style.interaction.selectable_labels = false;
            let size = |points: f32| points * text_scale;
            style.text_styles = [
                (TextStyle::Heading, FontId::proportional(size(17.0))),
                (TextStyle::Body, FontId::proportional(size(13.5))),
                (TextStyle::Button, FontId::proportional(size(13.5))),
                (TextStyle::Small, FontId::proportional(size(11.5))),
                (TextStyle::Monospace, FontId::monospace(size(12.5))),
            ]
            .into();
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.button_padding = egui::vec2(9.0, 4.0);
            style.spacing.interact_size.y = 24.0 * text_scale;
            style.spacing.slider_width = 96.0;
            let visuals = &mut style.visuals;
            visuals.panel_fill = ink.paper;
            visuals.window_fill = ink.card;
            visuals.extreme_bg_color = ink.card;
            visuals.faint_bg_color = ink.card;
            visuals.override_text_color = Some(ink.text);
            visuals.window_stroke = Stroke::new(1.0, ink.line);
            visuals.window_corner_radius = CornerRadius::same(10);
            visuals.menu_corner_radius = CornerRadius::same(8);
            visuals.selection.bg_fill = ink.chosen;
            visuals.selection.stroke = Stroke::new(1.0, ink.card);
            visuals.hyperlink_color = ink.chosen;
            visuals.warn_fg_color = ink.warn;
            visuals.error_fg_color = ink.block;
            for widget in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
                &mut visuals.widgets.open,
            ] {
                widget.corner_radius = CornerRadius::same(6);
            }
            visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, ink.line);
            visuals.widgets.inactive.weak_bg_fill = ink.card;
            visuals.widgets.inactive.bg_fill = ink.line;
            visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, ink.line);
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ink.chosen);
            visuals.widgets.active.bg_stroke = Stroke::new(1.5, ink.chosen);
        });
    }
}
