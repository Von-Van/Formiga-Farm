//! The stage in the middle, where the design is shown large and alive, and the strip beneath it:
//! how it moves, which way it faces, before and after, and how it will look on the desktop, at
//! home, and out on the hill.

use super::pictures::FrameKey;
use super::{FarmApp, View, style};
use crate::notices::Level;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, vec2};
use formiga_art::FRAME_SIZE;
use formiga_art::forms::{DesignRenderer, Intent};
use formiga_core::forms::Design;

const FRAME: f32 = FRAME_SIZE as f32;
const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

/// A short caption on a slip of paper, so it reads over the soil.
fn caption(ui: &egui::Ui, at: Pos2, text: &str) {
    let ink = style::ink(ui.visuals().dark_mode);
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::TextStyle::Small.resolve(ui.style()),
        ink.text,
    );
    let rect = Rect::from_center_size(
        Pos2::new(at.x, at.y + galley.size().y / 2.0 + 4.0),
        galley.size() + vec2(14.0, 8.0),
    );
    ui.painter().rect_filled(rect, 5.0, ink.card);
    ui.painter()
        .galley(rect.min + vec2(7.0, 4.0), galley, ink.text);
}

impl FarmApp {
    /// Which frame of the pose is showing now.
    fn frame_now(&self) -> u8 {
        if self.reduce_motion() {
            return 0;
        }
        let (frames, fps) = DesignRenderer::timing(self.intent);
        ((self.clock * f64::from(fps)) as u64 % u64::from(frames.max(1))) as u8
    }

    /// What to say under Formiga Hill or Home: `there`, or, for a sculpted form a trip leaves
    /// behind, that those apps draw it as its nearest companion body.
    fn travelling_caption(&self, there: &'static str) -> &'static str {
        if self.shown().form.sculpt().is_some() && !crate::drawing::TRIPS_CARRY_SCULPTS {
            "Formiga Hill and Home can't draw sculpted bodies yet,\nso it visits them as its nearest companion body, in its colours."
        } else {
            there
        }
    }

    /// Draw the shown design, `scale` physical pixels to an art pixel, standing with its ground
    /// row on `floor`, centred on `x`. Returns where the frame went. `hd` draws a sculpted form
    /// in Farm's own high definition; otherwise it is Desktop's own pixels, made larger. `travels`
    /// draws it as Formiga Hill and Home will ([`crate::drawing::as_it_travels`]).
    #[allow(clippy::too_many_arguments)]
    fn figure(
        &mut self,
        ui: &egui::Ui,
        x: f32,
        floor: f32,
        scale: f32,
        outlined: bool,
        hd: bool,
        travels: bool,
    ) -> Rect {
        let design = if travels {
            crate::drawing::as_it_travels(self.shown(), &self.base)
        } else {
            self.shown().clone()
        };
        let key = FrameKey {
            design: if self.before {
                u64::MAX
            } else {
                self.editor.changes
            },
            intent: self.intent,
            frame: self.frame_now(),
            facing_right: self.facing_right,
            outlined,
            detail: if hd { crate::drawing::DETAIL } else { 1 },
            travels,
            dressed: self.dressed && self.dress.is_some(),
        };
        self.paint_figure(ui, &design, key, x, floor, scale)
    }

    /// The design shown beside the workbench's to compare, drawn as the workbench's is on the
    /// stage, at `scale`, standing on `floor` and centred on `x`.
    fn compared_figure(&mut self, ui: &egui::Ui, design: &Design, x: f32, floor: f32, scale: f32) {
        let key = FrameKey {
            design: super::pictures::COMPARED,
            intent: self.intent,
            frame: self.frame_now(),
            facing_right: self.facing_right,
            outlined: false,
            detail: if self.grid { 1 } else { crate::drawing::DETAIL },
            travels: false,
            dressed: self.dressed && self.dress.is_some(),
        };
        self.paint_figure(ui, design, key, x, floor, scale);
    }

    /// Draw `design`'s frame named by `key`, `scale` physical pixels to an art pixel, standing
    /// with its ground row on `floor`, centred on `x`. Returns where the frame went.
    fn paint_figure(
        &mut self,
        ui: &egui::Ui,
        design: &Design,
        key: FrameKey,
        x: f32,
        floor: f32,
        scale: f32,
    ) -> Rect {
        let ctx = ui.ctx().clone();
        let points = scale / ctx.pixels_per_point();
        let baseline = DesignRenderer::resting_baseline(design, &self.base, self.reduce_motion());
        let texture = self.pictures.frame(
            &ctx,
            design,
            &self.base,
            key,
            self.editor.changes,
            self.reduce_motion(),
            self.dress,
        );
        let ground_row = FRAME - 1.0 - baseline as f32;
        let top_left = Pos2::new(
            (x - FRAME * points / 2.0).round(),
            (floor - (ground_row + 1.0) * points).round(),
        );
        let rect = Rect::from_min_size(top_left, vec2(FRAME * points, FRAME * points));
        ui.painter().image(texture.id(), rect, UV, Color32::WHITE);
        rect
    }

    pub(super) fn stage(&mut self, ui: &mut egui::Ui, area: Rect) {
        let ink = style::ink(ui.visuals().dark_mode);
        let ppp = ui.ctx().pixels_per_point();
        let room = area.shrink(18.0);
        if room.width() < 60.0 || room.height() < 60.0 {
            return;
        }
        match self.view {
            View::Stage => {
                // A chamber dug in the soil, lit, with the creature on its floor.
                let size = (room.width().min(room.height() * 1.15) * 0.84).min(680.0);
                let chamber = Rect::from_center_size(room.center(), vec2(size, size * 0.86));
                ui.painter().rect(
                    chamber,
                    size * 0.18,
                    ink.stage,
                    Stroke::new(3.0, ink.stage_edge),
                    egui::StrokeKind::Inside,
                );
                let compared = self.compare.clone();
                let room_for = if compared.is_some() { 0.5 } else { 0.78 };
                let scale = ((chamber.height() * ppp * room_for) / FRAME)
                    .floor()
                    .clamp(2.0, 14.0);
                let x = if compared.is_some() {
                    chamber.center().x + size * 0.22
                } else {
                    chamber.center().x
                };
                let floor = chamber.max.y - chamber.height() * 0.14;
                // The floor of the chamber.
                ui.painter().line_segment(
                    [
                        Pos2::new(chamber.min.x + size * 0.12, floor),
                        Pos2::new(chamber.max.x - size * 0.12, floor),
                    ],
                    Stroke::new(2.0, ink.stage_edge),
                );
                let rect = self.figure(ui, x, floor, scale, false, !self.grid, false);
                if let Some((name, design)) = &compared {
                    let other = chamber.center().x - size * 0.22;
                    self.compared_figure(ui, design, other, floor, scale);
                    for (at, label) in [(other, name.as_str()), (x, "Workbench")] {
                        ui.painter().text(
                            Pos2::new(at, floor + 10.0),
                            egui::Align2::CENTER_TOP,
                            label,
                            egui::TextStyle::Body.resolve(ui.style()),
                            ink.faint,
                        );
                    }
                }
                if self.grid {
                    let pixel = scale / ppp;
                    let line = Stroke::new(1.0, ink.line.gamma_multiply(0.35));
                    for i in 0..=FRAME_SIZE {
                        let at = i as f32 * pixel;
                        ui.painter().line_segment(
                            [
                                Pos2::new(rect.min.x + at, rect.min.y),
                                Pos2::new(rect.min.x + at, rect.max.y),
                            ],
                            line,
                        );
                        ui.painter().line_segment(
                            [
                                Pos2::new(rect.min.x, rect.min.y + at),
                                Pos2::new(rect.max.x, rect.min.y + at),
                            ],
                            line,
                        );
                    }
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, ink.stage_edge),
                        egui::StrokeKind::Outside,
                    );
                }
                if self.before {
                    ui.painter().text(
                        Pos2::new(chamber.center().x, chamber.min.y + 18.0),
                        egui::Align2::CENTER_TOP,
                        "Before",
                        egui::TextStyle::Heading.resolve(ui.style()),
                        ink.faint,
                    );
                }
            }
            View::Desktop => {
                // A strip of desktop: a window's edge, a dock along the bottom, at the size it
                // will live at.
                let screen = Rect::from_center_size(
                    room.center(),
                    vec2(room.width().min(760.0), room.height().min(420.0)),
                );
                let paper = Color32::from_rgb(0x9f, 0xb8, 0xc9);
                ui.painter().rect_filled(screen, 8.0, paper);
                let window = Rect::from_min_size(
                    screen.min + vec2(screen.width() * 0.08, screen.height() * 0.12),
                    vec2(screen.width() * 0.5, screen.height() * 0.5),
                );
                ui.painter()
                    .rect_filled(window, 6.0, Color32::from_rgb(0xf7, 0xf5, 0xf0));
                ui.painter().rect_filled(
                    Rect::from_min_size(window.min, vec2(window.width(), 18.0)),
                    egui::CornerRadius {
                        nw: 6,
                        ne: 6,
                        sw: 0,
                        se: 0,
                    },
                    Color32::from_rgb(0xdd, 0xd8, 0xd0),
                );
                let dock =
                    Rect::from_min_max(Pos2::new(screen.min.x, screen.max.y - 26.0), screen.max);
                ui.painter().rect_filled(
                    dock,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 8,
                        se: 8,
                    },
                    Color32::from_rgb(0x4a, 0x55, 0x60),
                );
                // Its own size, as Desktop draws it, and perched on the window's top edge.
                self.figure(
                    ui,
                    screen.min.x + screen.width() * 0.72,
                    dock.min.y,
                    2.0,
                    true,
                    false,
                    false,
                );
                self.figure(
                    ui,
                    window.min.x + window.width() * 0.7,
                    window.min.y,
                    2.0,
                    true,
                    false,
                    false,
                );
                caption(
                    ui,
                    Pos2::new(screen.center().x, screen.max.y + 6.0),
                    "At the size it lives at on the desktop, outlined as Desktop draws it.",
                );
            }
            View::Home => {
                // A patch of floorboards, seen from above at an angle, as Formiga Home sets out
                // its rooms.
                let center = room.center() + vec2(0.0, room.height() * 0.12);
                let (w, h) = (
                    room.width().min(520.0) * 0.42,
                    room.width().min(520.0) * 0.21,
                );
                let wood = [
                    Color32::from_rgb(0xc8, 0x9a, 0x6a),
                    Color32::from_rgb(0xb8, 0x8a, 0x5c),
                ];
                let diamond = |dx: f32, dy: f32, s: f32| {
                    vec![
                        Pos2::new(center.x + dx, center.y + dy - h * s),
                        Pos2::new(center.x + dx + w * s, center.y + dy),
                        Pos2::new(center.x + dx, center.y + dy + h * s),
                        Pos2::new(center.x + dx - w * s, center.y + dy),
                    ]
                };
                ui.painter().add(egui::Shape::convex_polygon(
                    diamond(0.0, 0.0, 1.0),
                    wood[0],
                    Stroke::new(2.0, Color32::from_rgb(0x7a, 0x52, 0x36)),
                ));
                for i in 1..6 {
                    let t = i as f32 / 6.0;
                    let a = Pos2::new(center.x - w + w * t, center.y - h * t);
                    let b = Pos2::new(center.x + w * t, center.y + h - h * t);
                    ui.painter().line_segment([a, b], Stroke::new(1.0, wood[1]));
                }
                self.figure(ui, center.x, center.y, 4.0, false, false, true);
                caption(
                    ui,
                    Pos2::new(center.x, center.y + h + 10.0),
                    self.travelling_caption("Standing on its own spot in Formiga Home."),
                );
            }
            View::Hill => {
                let field = Rect::from_center_size(
                    room.center(),
                    vec2(room.width().min(720.0), room.height().min(420.0)),
                );
                ui.painter()
                    .rect_filled(field, 10.0, Color32::from_rgb(0xcf, 0xe6, 0xf0));
                let ground = field.min.y + field.height() * 0.66;
                let hill: Vec<Pos2> = (0..=24)
                    .map(|i| {
                        let t = i as f32 / 24.0;
                        Pos2::new(
                            field.min.x + field.width() * t,
                            ground - (t * std::f32::consts::PI).sin() * field.height() * 0.08,
                        )
                    })
                    .chain([
                        Pos2::new(field.max.x, field.max.y),
                        Pos2::new(field.min.x, field.max.y),
                    ])
                    .collect();
                ui.painter().add(egui::Shape::convex_polygon(
                    hill,
                    Color32::from_rgb(0x8c, 0xb8, 0x62),
                    Stroke::NONE,
                ));
                let floor = ground - field.height() * 0.08 + 1.0;
                self.figure(ui, field.center().x, floor, 3.0, false, false, true);
                caption(
                    ui,
                    Pos2::new(field.center().x, field.max.y + 6.0),
                    self.travelling_caption("Out on the grass of Formiga Hill."),
                );
            }
        }
    }

    pub(super) fn strip(&mut self, ui: &mut egui::Ui) {
        let ink = style::ink(ui.visuals().dark_mode);
        let notices = self.notices().to_vec();
        let shown = if self.all_notes { notices.len() } else { 3 };
        for notice in notices.iter().take(shown) {
            let (color, mark) = match notice.level {
                Level::Blocks => (ink.block, "\u{25cf}"),
                Level::Warning => (ink.warn, "\u{25b2}"),
                Level::Note => (ink.faint, "\u{2022}"),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(mark).color(color));
                ui.label(egui::RichText::new(&notice.text).color(color));
            });
        }
        if notices.len() > 3 {
            let toggle = if self.all_notes {
                "Fewer notes".to_owned()
            } else {
                format!("{} more notes", notices.len() - 3)
            };
            if ui.small_button(toggle).clicked() {
                self.all_notes = !self.all_notes;
            }
        }
        ui.horizontal_wrapped(|ui| {
            for intent in Intent::ALL {
                if ui
                    .selectable_label(self.intent == intent, intent.label())
                    .clicked()
                {
                    self.intent = intent;
                }
            }
            ui.separator();
            if ui
                .button(if self.facing_right {
                    "Facing right"
                } else {
                    "Facing left"
                })
                .on_hover_text("Turn round. F")
                .clicked()
            {
                self.facing_right = !self.facing_right;
            }
            if let Some(dress) = self.dress {
                let what = dress.accessory.label().to_lowercase();
                ui.checkbox(&mut self.dressed, format!("Wearing their {what}"))
                    .on_hover_text(
                        "Show what they wear in Desktop on the new form, as Desktop will place it.",
                    );
            }
            let changed = *self.editor.design() != self.editor.arrival;
            let label = if self.snapshot.creature().is_some() {
                "Before"
            } else {
                "Started from"
            };
            if ui
                .add_enabled(
                    changed || self.before,
                    egui::Button::selectable(self.before, label),
                )
                .on_hover_text("Show how it looked before, to compare. B")
                .clicked()
            {
                self.before = !self.before;
            }
            if let Some((name, _)) = &self.compare
                && ui
                    .button(format!("Stop comparing with {name}"))
                    .on_hover_text(
                        "Compare any draft or preset from its \u{2026} menu on the shelves.",
                    )
                    .clicked()
            {
                self.compare = None;
                self.pictures.forget_compared();
            }
        });
        ui.horizontal_wrapped(|ui| {
            for view in View::ALL {
                if ui
                    .selectable_label(self.view == view, view.label())
                    .clicked()
                {
                    self.view = view;
                }
            }
            if self.view == View::Stage {
                ui.separator();
                ui.checkbox(&mut self.grid, "Desktop's pixels")
                    .on_hover_text(
                        "Show it pixel by pixel, at the 48 pixels Desktop draws it with, \
                     instead of Farm's own smooth drawing.",
                    );
            }
        });
    }
}
