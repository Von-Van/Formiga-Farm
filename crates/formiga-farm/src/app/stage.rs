//! The stage in the middle, where the design is shown large and alive, and the strip beneath it:
//! how it moves, which way it faces, before and after, and how it will look on the desktop, at
//! home, and out on the hill.

use super::pictures::FrameKey;
use super::{FarmApp, View, style};
use crate::notices::Level;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, vec2};
use formiga_art::FRAME_SIZE;
use formiga_forms::{DesignRenderer, Intent};

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

    /// Draw the shown design, `scale` physical pixels to an art pixel, standing with its ground
    /// row on `floor`, centred on `x`. Returns where the frame went.
    fn figure(&mut self, ui: &egui::Ui, x: f32, floor: f32, scale: f32, outlined: bool) -> Rect {
        let ctx = ui.ctx().clone();
        let points = scale / ctx.pixels_per_point();
        let design = self.shown().clone();
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
        };
        let baseline = DesignRenderer::resting_baseline(&design, &self.base, self.reduce_motion());
        let texture = self.pictures.frame(
            &ctx,
            &design,
            &self.base,
            key,
            self.editor.changes,
            self.reduce_motion(),
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
                let scale = ((chamber.height() * ppp * 0.78) / FRAME)
                    .floor()
                    .clamp(2.0, 14.0);
                let floor = chamber.max.y - chamber.height() * 0.14;
                // The floor of the chamber.
                ui.painter().line_segment(
                    [
                        Pos2::new(chamber.min.x + size * 0.12, floor),
                        Pos2::new(chamber.max.x - size * 0.12, floor),
                    ],
                    Stroke::new(2.0, ink.stage_edge),
                );
                let rect = self.figure(ui, chamber.center().x, floor, scale, false);
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
                );
                self.figure(
                    ui,
                    window.min.x + window.width() * 0.7,
                    window.min.y,
                    2.0,
                    true,
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
                self.figure(ui, center.x, center.y, 4.0, false);
                caption(
                    ui,
                    Pos2::new(center.x, center.y + h + 10.0),
                    "Standing on its own spot in Formiga Home.",
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
                self.figure(ui, field.center().x, floor, 3.0, false);
                caption(
                    ui,
                    Pos2::new(field.center().x, field.max.y + 6.0),
                    "Out on the grass of Formiga Hill.",
                );
            }
        }
    }

    pub(super) fn strip(&mut self, ui: &mut egui::Ui) {
        let ink = style::ink(ui.visuals().dark_mode);
        let notices = self.notices().to_vec();
        for notice in notices.iter().take(3) {
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
                ui.checkbox(&mut self.grid, "Pixel grid");
            }
        });
    }
}
