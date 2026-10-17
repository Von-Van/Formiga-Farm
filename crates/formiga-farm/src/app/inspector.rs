//! The inspector along the right: one page for each layer of the design. Named sliders and
//! choices rather than numbers, each within the range its plan allows, and every page can be
//! rolled at random or put back on its own.

use super::{FarmApp, style};
use crate::editor::Layer;
use eframe::egui;
use formiga_core::forms::{
    Design, Dimension, Form, Ink, MAX_MARKINGS, Marking, MarkingKind, NUDGE, Part, Plan, STEPS,
    Sculpt, Slot, Treatment,
};
use formiga_core::{
    BodyPlan, BrowStyle, CheekStyle, CreatureDesign, EarStyle, EyeShape, HighlightStyle,
    MouthStyle, PupilStyle,
};

const COMPANIONS: [(BodyPlan, &str); 5] = [
    (BodyPlan::Round, "Round"),
    (BodyPlan::Upright, "Upright"),
    (BodyPlan::Long, "Four-pawed"),
    (BodyPlan::Winged, "Winged"),
    (BodyPlan::Blob, "Blob"),
];

/// A small labelled row.
fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let height = ui.spacing().interact_size.y;
        ui.allocate_ui_with_layout(
            egui::vec2(96.0, height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(96.0);
                ui.add(egui::Label::new(label).truncate());
            },
        );
        add(ui);
    });
}

fn slider<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut N,
    low: N,
    high: N,
) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        changed = ui.add(egui::Slider::new(value, low..=high)).changed();
    });
    changed
}

fn choose<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        let shown = options
            .iter()
            .find(|(option, _)| option == value)
            .map_or("", |(_, name)| *name);
        egui::ComboBox::from_id_salt(id)
            .selected_text(shown)
            .width(132.0)
            .show_ui(ui, |ui| {
                for (option, name) in options {
                    if ui.selectable_label(*value == *option, *name).clicked() && *value != *option
                    {
                        *value = *option;
                        changed = true;
                    }
                }
            });
    });
    changed
}

/// A colour's well, with a menu of `swatches` beside it to take one from.
fn color(ui: &mut egui::Ui, label: &str, value: &mut [u8; 3], swatches: &[[u8; 3]]) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        changed = ui
            .color_edit_button_srgb(value)
            .on_hover_text(format!("{label}: {}", hex(*value)))
            .changed();
        if swatches.is_empty() {
            return;
        }
        ui.menu_button("\u{2026}", |ui| {
            ui.label(
                egui::RichText::new("This design\u{2019}s colours, then your palettes\u{2019}")
                    .small(),
            );
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(184.0);
                for &swatch in swatches {
                    if swatch_button(ui, swatch, *value == swatch).clicked() {
                        *value = swatch;
                        changed = true;
                        ui.close();
                    }
                }
            });
        })
        .response
        .on_hover_text(format!("Swatches for {}", label.to_lowercase()));
    });
    changed
}

/// A colour as `#rrggbb`.
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn swatch_button(ui: &mut egui::Ui, rgb: [u8; 3], chosen: bool) -> egui::Response {
    let ink = style::ink(ui.visuals().dark_mode);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
    let edge = if chosen || response.hovered() {
        egui::Stroke::new(2.0, ink.text)
    } else {
        egui::Stroke::new(1.0, ink.line)
    };
    ui.painter().rect(
        rect,
        3.0,
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        edge,
        egui::StrokeKind::Inside,
    );
    response.on_hover_text(hex(rgb))
}

/// A small picture to choose by: `texture`, `size` frame pixels across, in a tile that shows
/// whether it is the one chosen.
fn picture_choice(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    size: egui::Vec2,
    chosen: bool,
    hover: &str,
) -> egui::Response {
    let ink = style::ink(ui.visuals().dark_mode);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
    let edge = if chosen {
        egui::Stroke::new(2.0, ink.chosen)
    } else if response.hovered() {
        egui::Stroke::new(1.5, ink.text)
    } else {
        egui::Stroke::new(1.0, ink.line)
    };
    ui.painter()
        .rect(rect, 5.0, ink.card, edge, egui::StrokeKind::Inside);
    // Whole pixels only, so the picture stays crisp: as large as fits.
    let ppp = ui.ctx().pixels_per_point();
    let fit = ((40.0 * ppp) / size.x.max(size.y).max(1.0))
        .floor()
        .max(1.0)
        / ppp;
    let image = egui::Rect::from_center_size(rect.center(), size * fit);
    ui.painter().image(
        texture.id(),
        image,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    response.on_hover_text(hover)
}

/// A palette's colours in a row, to see at a glance.
fn strip(ui: &mut egui::Ui, colors: &[[u8; 3]]) {
    let ink = style::ink(ui.visuals().dark_mode);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(10.0 * colors.len() as f32, 14.0),
        egui::Sense::hover(),
    );
    for (index, rgb) in colors.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            rect.min + egui::vec2(10.0 * index as f32, 0.0),
            egui::vec2(10.0, 14.0),
        );
        ui.painter()
            .rect_filled(cell, 0.0, egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
    }
    ui.painter().rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, ink.line),
        egui::StrokeKind::Outside,
    );
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).strong());
}

impl FarmApp {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        let ink = style::ink(ui.visuals().dark_mode);
        ui.horizontal(|ui| {
            for layer in Layer::ALL {
                if ui
                    .selectable_label(
                        self.layer == layer,
                        egui::RichText::new(layer.label()).strong(),
                    )
                    .clicked()
                {
                    self.layer = layer;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button(format!("Roll {}", self.layer.label().to_lowercase()))
                .on_hover_text("Roll this layer at random, and only this one.")
                .clicked()
            {
                self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) % 100_000;
                self.editor.randomize(self.layer, self.seed);
            }
            if ui
                .button("Put back")
                .on_hover_text("Put this layer back as it started.")
                .clicked()
            {
                self.editor.reset_layer(self.layer);
            }
            ui.label(
                egui::RichText::new(format!("roll {}", self.seed))
                    .small()
                    .color(ink.faint),
            )
            .on_hover_text("The last roll's number, so a roll can be told apart from another.");
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Roll everything")
                .on_hover_text(
                    "Roll a new body and every layer at once, but what is ticked to keep. It undoes as one.",
                )
                .clicked()
            {
                self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) % 100_000;
                self.editor.roll_all(self.locks, self.seed);
            }
            ui.label(egui::RichText::new("Keeping").small().color(ink.faint));
            ui.checkbox(&mut self.locks.body, "Body")
                .on_hover_text("Keep the body: which plan, or which of Desktop's bodies.");
            for layer in Layer::ALL {
                ui.checkbox(self.locks.keep(layer), layer.label())
                    .on_hover_text(format!(
                        "Keep {} as it is when rolling everything.",
                        layer.label().to_lowercase()
                    ));
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let design = self.editor.design().clone();
                match self.layer {
                    Layer::Form => self.form_page(ui, &design),
                    Layer::Parts => self.parts_page(ui, &design),
                    Layer::Finish => self.finish_page(ui, &design),
                    Layer::Face => self.face_page(ui, &design),
                }
                ui.add_space(12.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button("Start over")
                        .on_hover_text("Back to what this design was last started from.")
                        .clicked()
                    {
                        self.editor.reset_to_start();
                    }
                    if self.snapshot.creature().is_some()
                        && ui
                            .button("Restore their look")
                            .on_hover_text("Back to how they looked when Farm opened.")
                            .clicked()
                    {
                        self.editor.restore_arrival();
                    }
                });
            });
    }

    fn form_page(&mut self, ui: &mut egui::Ui, design: &Design) {
        let ink = style::ink(ui.visuals().dark_mode);
        heading(ui, "Body");
        ui.label(
            egui::RichText::new("Desktop's companion bodies")
                .small()
                .color(ink.faint),
        );
        ui.horizontal_wrapped(|ui| {
            for (body, label) in COMPANIONS {
                let chosen = design.form.recipe().is_some_and(|r| r.body == body);
                if ui.selectable_label(chosen, label).clicked() && !chosen {
                    self.editor.set_companion(body);
                }
            }
        });
        ui.label(
            egui::RichText::new("Sculpted bodies")
                .small()
                .color(ink.faint),
        );
        ui.horizontal_wrapped(|ui| {
            for plan in Plan::ALL {
                let chosen = design.form.sculpt().is_some_and(|s| s.plan == plan);
                let response = ui.selectable_label(chosen, plan.label());
                let response = response.on_hover_text(match plan.locomotion() {
                    formiga_core::forms::Locomotion::Walk => "Walks on four legs.",
                    formiga_core::forms::Locomotion::Amble => {
                        "Stands and ambles upright, with arms."
                    }
                    formiga_core::forms::Locomotion::Float => {
                        "Floats a little above the ground and bobs along; never walks."
                    }
                    formiga_core::forms::Locomotion::Crawl => "Paddles along low to the ground.",
                    formiga_core::forms::Locomotion::Hop => {
                        "Hops on two feet, with a flap now and then."
                    }
                });
                if response.clicked() && !chosen {
                    self.editor.set_plan(plan);
                }
            }
        });
        match &design.form {
            Form::Original => {
                ui.add_space(6.0);
                ui.label(
                    "This is their original look, from before companions had recipes. Its face can change here; choose a body above to reshape the rest.",
                );
            }
            Form::Companion { recipe } => {
                heading(ui, "Proportions");
                let mut r = *recipe;
                let mut key = None;
                if slider(ui, "Width", &mut r.width, 8, 12) {
                    key = Some("recipe-width");
                }
                if slider(ui, "Height", &mut r.height, 7, 11) {
                    key = Some("recipe-height");
                }
                if slider(ui, "Head size", &mut r.head, 7, 9) {
                    key = Some("recipe-head");
                }
                if slider(ui, "Leg length", &mut r.legs, 3, 6) {
                    key = Some("recipe-legs");
                }
                if let Some(key) = key {
                    self.editor
                        .edit(Some(key), |d| d.form = Form::Companion { recipe: r });
                }
            }
            Form::Sculpted { sculpt } => {
                heading(ui, "Proportions");
                let mut shape = sculpt.shape;
                let mut changed = None;
                for dimension in Dimension::ALL {
                    if !sculpt.plan.uses(dimension) {
                        continue;
                    }
                    let mut value = shape.get(dimension);
                    if slider(ui, dimension.label(), &mut value, 0, STEPS) {
                        shape.set(dimension, value);
                        changed = Some(dimension);
                    }
                }
                if let Some(dimension) = changed {
                    let key = format!("shape-{dimension:?}");
                    self.editor.edit(Some(&key), |d| {
                        if let Form::Sculpted { sculpt } = &mut d.form {
                            sculpt.shape = shape;
                        }
                    });
                }
            }
        }
    }

    fn parts_page(&mut self, ui: &mut egui::Ui, design: &Design) {
        match &design.form {
            Form::Sculpted { sculpt } => self.sculpt_parts(ui, design, sculpt),
            Form::Companion { recipe } => self.recipe_parts(ui, recipe),
            Form::Original => {
                ui.label("Choose a body under Form to fit parts to it.");
            }
        }
    }

    fn sculpt_parts(&mut self, ui: &mut egui::Ui, design: &Design, sculpt: &Sculpt) {
        let ink = style::ink(ui.visuals().dark_mode);
        for slot in Slot::ALL {
            if !sculpt.plan.has(slot) {
                continue;
            }
            let fitted = sculpt.part(slot).copied();
            egui::Frame::new()
                .fill(ink.card)
                .stroke(egui::Stroke::new(1.0, ink.line))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut kind = fitted.map(|p| p.kind);
                    let mut options: Vec<(Option<formiga_core::forms::PartKind>, &str)> =
                        vec![(None, "None")];
                    options.extend(slot.kinds().map(|k| (Some(k), k.label())));
                    if self.choose_part(ui, design, slot, fitted, &mut kind, &options) {
                        self.editor.edit(None, |d| {
                            if let Form::Sculpted { sculpt } = &mut d.form {
                                match kind {
                                    Some(kind) => {
                                        let ink = fitted.map_or(Ink::Primary, |p| p.ink);
                                        let size =
                                            fitted.map_or(formiga_core::forms::MIDDLE, |p| p.size);
                                        sculpt.fit(Part::new(kind, ink).sized(size));
                                    }
                                    None => {
                                        sculpt.remove(slot);
                                    }
                                }
                            }
                        });
                    }
                    let Some(part) = fitted else {
                        return;
                    };
                    let mut p = part;
                    let mut key = None;
                    if slider(ui, "Size", &mut p.size, 0, STEPS) {
                        key = Some("size");
                    }
                    let inks: Vec<(Ink, &str)> = Ink::ALL.iter().map(|i| (*i, i.label())).collect();
                    if choose(ui, "Colour", &format!("ink-{slot:?}"), &mut p.ink, &inks) {
                        key = Some("ink");
                    }
                    egui::CollapsingHeader::new("Placement")
                        .id_salt(("placement", slot))
                        .show(ui, |ui| {
                            if slider(ui, "Tilt", &mut p.tilt, -NUDGE, NUDGE) {
                                key = Some("tilt");
                            }
                            if slider(ui, "Lift", &mut p.lift, -NUDGE, NUDGE) {
                                key = Some("lift");
                            }
                            if slot.paired() {
                                if slider(ui, "Uneven", &mut p.uneven, -NUDGE, NUDGE) {
                                    key = Some("uneven");
                                }
                                if p.uneven != 0 && ui.small_button("Restore symmetry").clicked() {
                                    p.uneven = 0;
                                    key = Some("symmetry");
                                }
                            }
                        });
                    if let Some(key) = key {
                        let key = format!("part-{slot:?}-{key}");
                        self.editor.edit(Some(&key), |d| {
                            if let Form::Sculpted { sculpt } = &mut d.form {
                                sculpt.fit(p);
                            }
                        });
                    }
                });
            ui.add_space(4.0);
        }
        if !self.editor.set_aside().is_empty() {
            ui.label(
                egui::RichText::new("Some parts are set aside: this body has nowhere for them. They come back if it changes back.")
                    .small()
                    .color(ink.faint),
            );
        }
    }

    /// Which part goes in `slot`, chosen from a list that pictures the design wearing each.
    /// Whether the choice changed.
    fn choose_part(
        &mut self,
        ui: &mut egui::Ui,
        design: &Design,
        slot: Slot,
        fitted: Option<Part>,
        kind: &mut Option<formiga_core::forms::PartKind>,
        options: &[(Option<formiga_core::forms::PartKind>, &str)],
    ) -> bool {
        let mut changed = false;
        let ctx = ui.ctx().clone();
        row(ui, slot.label(), |ui| {
            let shown = options
                .iter()
                .find(|(option, _)| option == kind)
                .map_or("", |(_, name)| *name);
            egui::ComboBox::from_id_salt(("slot", slot))
                .selected_text(shown)
                .width(132.0)
                .height(360.0)
                .show_ui(ui, |ui| {
                    for &(option, name) in options {
                        let mut variant = design.clone();
                        if let Form::Sculpted { sculpt } = &mut variant.form {
                            match option {
                                Some(option) => {
                                    let ink = fitted.map_or(Ink::Primary, |p| p.ink);
                                    let size =
                                        fitted.map_or(formiga_core::forms::MIDDLE, |p| p.size);
                                    sculpt.fit(Part::new(option, ink).sized(size));
                                }
                                None => {
                                    sculpt.remove(slot);
                                }
                            }
                        }
                        let (texture, size) = self.pictures.variant(
                            &ctx,
                            &format!("part-{slot:?}-{option:?}"),
                            &variant,
                            &self.base,
                            self.editor.changes,
                            false,
                        );
                        let picked = ui
                            .horizontal(|ui| {
                                let tile =
                                    picture_choice(ui, &texture, size, *kind == option, name);
                                let label = ui.selectable_label(*kind == option, name);
                                tile.clicked() || label.clicked()
                            })
                            .inner;
                        if picked && *kind != option {
                            *kind = option;
                            changed = true;
                        }
                    }
                });
        });
        changed
    }

    fn recipe_parts(&mut self, ui: &mut egui::Ui, recipe: &CreatureDesign) {
        let mut r = *recipe;
        let mut key: Option<&str> = None;
        heading(ui, "Head");
        let ears = [
            (EarStyle::None, "None"),
            (EarStyle::Round, "Round"),
            (EarStyle::Pointed, "Pointed"),
            (EarStyle::Long, "Long"),
            (EarStyle::Floppy, "Floppy"),
            (EarStyle::Tuft, "Tufts"),
        ];
        if choose(ui, "Ears", "recipe-ears", &mut r.ears, &ears) {
            key = Some("ears");
        }
        if slider(ui, "Ear size", &mut r.ear_size, 3, 7) {
            key = Some("ear-size");
        }
        if choose(
            ui,
            "Muzzle",
            "recipe-muzzle",
            &mut r.muzzle,
            &[(0, "None"), (1, "Small"), (2, "Wide")],
        ) {
            key = Some("muzzle");
        }
        if choose(
            ui,
            "Crown",
            "recipe-crown",
            &mut r.classic.crown,
            &[(0, "Its ears"), (1, "Antennae"), (2, "Sprouts")],
        ) {
            key = Some("crown");
        }
        if r.archetype > 0
            && choose(
                ui,
                "Horns",
                "recipe-horns",
                &mut r.details.horns,
                &[(0, "None"), (1, "Nubs"), (2, "Swept back")],
            )
        {
            key = Some("horns");
        }
        heading(ui, "Body");
        if choose(
            ui,
            "Tail",
            "recipe-tail",
            &mut r.tail,
            &[
                (0, "None"),
                (1, "Puff"),
                (2, "Taper"),
                (3, "Tufted taper"),
                (4, "Big and round"),
            ],
        ) {
            key = Some("tail");
        }
        if choose(
            ui,
            "Odd tail",
            "recipe-classic-tail",
            &mut r.classic.tail,
            &[(0, "None"), (1, "Curl"), (2, "Star")],
        ) {
            key = Some("classic-tail");
        }
        if choose(
            ui,
            "Limbs",
            "recipe-limbs",
            &mut r.classic.limbs,
            &[(0, "Paws"), (1, "Nubs"), (2, "Stick legs")],
        ) {
            key = Some("limbs");
        }
        if r.archetype > 0 {
            if choose(
                ui,
                "Wing-nubs",
                "recipe-wings",
                &mut r.details.wings,
                &[(0, "None"), (1, "Membrane"), (2, "Feathered")],
            ) {
                key = Some("wings");
            }
            if choose(
                ui,
                "Tail tip",
                "recipe-tip",
                &mut r.details.tip,
                &[(0, "None"), (1, "Flame"), (2, "Bobble")],
            ) {
                if r.details.tip > 0 && !matches!(r.tail, 2 | 3) {
                    r.tail = 2;
                }
                if r.details.tip > 0 && r.details.tip_color == [0; 3] {
                    r.details.tip_color = r.accent;
                }
                key = Some("tip");
            }
        }
        if let Some(key) = key {
            let key = format!("recipe-{key}");
            self.editor
                .edit(Some(&key), |d| d.form = Form::Companion { recipe: r });
        }
    }

    fn finish_page(&mut self, ui: &mut egui::Ui, design: &Design) {
        let swatches = crate::palettes::swatches(
            design,
            self.palettes
                .iter()
                .map(|palette| palette.colors.as_slice()),
            24,
        );
        if !matches!(design.form, Form::Original) {
            self.palette_row(ui, design);
        }
        match &design.form {
            Form::Sculpted { sculpt } => self.sculpt_finish(ui, design, sculpt, &swatches),
            Form::Companion { recipe } => {
                let mut r = *recipe;
                let mut key = None;
                heading(ui, "Colours");
                if color(ui, "Coat", &mut r.coat, &swatches) {
                    key = Some("coat");
                }
                if color(ui, "Accent", &mut r.accent, &swatches) {
                    key = Some("accent");
                }
                let mut candy = r.classic.coat > 0;
                row(ui, "Candy coat", |ui| {
                    if ui.checkbox(&mut candy, "").changed() {
                        r.classic.coat = u8::from(candy);
                        key = Some("candy");
                    }
                });
                heading(ui, "Markings");
                if choose(
                    ui,
                    "Marking",
                    "recipe-marking",
                    &mut r.marking,
                    &[
                        (0, "None"),
                        (1, "Pale belly"),
                        (6, "Two pale spots"),
                        (5, "Band"),
                        (4, "Patch"),
                        (2, "Stripes"),
                        (3, "Spots"),
                    ],
                ) {
                    key = Some("marking");
                }
                if choose(
                    ui,
                    "Pattern",
                    "recipe-pattern",
                    &mut r.classic.pattern,
                    &[(0, "None"), (1, "Stripes"), (2, "Spots"), (3, "Patches")],
                ) {
                    key = Some("pattern");
                }
                if r.archetype > 0 {
                    let mut belly = r.details.belly > 0;
                    row(ui, "Belly patch", |ui| {
                        if ui.checkbox(&mut belly, "").changed() {
                            r.details.belly = u8::from(belly);
                            if belly && r.details.belly_color == [0; 3] {
                                r.details.belly_color = [0xf6, 0xe6, 0xc8];
                            }
                            key = Some("belly");
                        }
                    });
                    if r.details.belly > 0
                        && color(ui, "Belly colour", &mut r.details.belly_color, &swatches)
                    {
                        key = Some("belly-color");
                    }
                    if r.details.tip > 0
                        && color(ui, "Tip colour", &mut r.details.tip_color, &swatches)
                    {
                        key = Some("tip-color");
                    }
                }
                if let Some(key) = key {
                    let key = format!("recipe-finish-{key}");
                    self.editor
                        .edit(Some(&key), |d| d.form = Form::Companion { recipe: r });
                }
            }
            Form::Original => {
                ui.label("Their original colours come with their original look. Choose a body under Form to recolour them.");
            }
        }
    }

    /// Put a palette on the design, or keep its colours as one.
    fn palette_row(&mut self, ui: &mut egui::Ui, design: &Design) {
        let mut put_on = None;
        let mut forget = None;
        let mut keep = false;
        row(ui, "Palette", |ui| {
            ui.menu_button("Use one\u{2026}", |ui| {
                ui.set_max_height(360.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if !self.palettes.is_empty() {
                        ui.label(egui::RichText::new("Yours").strong());
                        for palette in &self.palettes {
                            ui.horizontal(|ui| {
                                strip(ui, &palette.colors);
                                if ui.button(&palette.name).clicked() {
                                    put_on = Some(palette.colors.clone());
                                    ui.close();
                                }
                                if ui
                                    .small_button("\u{00d7}")
                                    .on_hover_text("Forget this palette.")
                                    .clicked()
                                {
                                    forget = Some(palette.id.clone());
                                }
                            });
                        }
                        ui.separator();
                    }
                    ui.label(egui::RichText::new("From the animals").strong());
                    for (name, colors) in crate::palettes::animals(&self.presets) {
                        ui.horizontal(|ui| {
                            strip(ui, &colors);
                            if ui.button(name).clicked() {
                                put_on = Some(colors.clone());
                                ui.close();
                            }
                        });
                    }
                });
            })
            .response
            .on_hover_text("Put another design's colours on this one. Only the colours change.");
            keep = ui
                .button("Keep these")
                .on_hover_text("Keep these colours as a palette, to put on another design.")
                .clicked();
        });
        if let Some(colors) = put_on {
            self.editor
                .edit(None, |d| *d = crate::palettes::with_colors(d, &colors));
        }
        if let Some(id) = forget {
            self.store.delete_palette(&id);
            self.palettes = self.store.palettes();
        }
        if keep {
            let palette =
                crate::store::Palette::new(&self.draft.name, crate::palettes::colors_of(design));
            match self.store.save_palette(&palette) {
                Ok(()) => {
                    self.palettes = self.store.palettes();
                    self.say(
                        format!("Kept these colours as \u{201c}{}\u{201d}.", palette.name),
                        false,
                    );
                }
                Err(error) => self.say(format!("Could not keep these colours: {error}"), true),
            }
        }
    }

    fn sculpt_finish(
        &mut self,
        ui: &mut egui::Ui,
        design: &Design,
        sculpt: &Sculpt,
        swatches: &[[u8; 3]],
    ) {
        let ink = style::ink(ui.visuals().dark_mode);
        let mut coat = sculpt.coat;
        let mut key = None;
        heading(ui, "Coat");
        let treatments: Vec<(Treatment, &str)> =
            Treatment::ALL.iter().map(|t| (*t, t.label())).collect();
        if choose(
            ui,
            "Treatment",
            "treatment",
            &mut coat.treatment,
            &treatments,
        ) {
            key = Some("treatment");
        }
        for (ink, label) in [
            (Ink::Primary, "Primary"),
            (Ink::Secondary, "Secondary"),
            (Ink::Underside, "Underside"),
            (Ink::Accent, "Accent"),
            (Ink::Feature, "Horns & claws"),
        ] {
            let mut value = coat.ink(ink);
            if color(ui, label, &mut value, swatches) {
                coat.set_ink(ink, value);
                key = Some(label);
            }
        }
        if color(ui, "Eyes", &mut coat.eyes, swatches) {
            key = Some("eyes");
        }
        if slider(ui, "Belly reach", &mut coat.underside_reach, 0, STEPS) {
            key = Some("reach");
        }
        if let Some(key) = key {
            let key = format!("coat-{key}");
            self.editor.edit(Some(&key), |d| {
                if let Form::Sculpted { sculpt } = &mut d.form {
                    sculpt.coat = coat;
                }
            });
        }
        heading(ui, "Markings");
        let kinds: Vec<(MarkingKind, &str)> =
            MarkingKind::ALL.iter().map(|k| (*k, k.label())).collect();
        let mut markings = sculpt.markings.clone();
        let mut key = None;
        let mut remove = None;
        for (index, marking) in markings.iter_mut().enumerate() {
            egui::Frame::new()
                .fill(ink.card)
                .stroke(egui::Stroke::new(1.0, ink.line))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if choose(
                        ui,
                        "Kind",
                        &format!("marking-kind-{index}"),
                        &mut marking.kind,
                        &kinds,
                    ) {
                        key = Some(format!("marking-{index}-kind"));
                    }
                    if color(ui, "Colour", &mut marking.color, swatches) {
                        key = Some(format!("marking-{index}-color"));
                    }
                    if slider(ui, "Amount", &mut marking.amount, 0, STEPS) {
                        key = Some(format!("marking-{index}-amount"));
                    }
                    if slider(ui, "Size", &mut marking.size, 0, STEPS) {
                        key = Some(format!("marking-{index}-size"));
                    }
                    row(ui, "Layout", |ui| {
                        if ui
                            .button("Shuffle")
                            .on_hover_text("Lay this marking out another way.")
                            .clicked()
                        {
                            self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345)
                                % 100_000;
                            marking.layout =
                                marking.layout.wrapping_add(1 + (self.seed % 254) as u8);
                            key = Some(format!("marking-{index}-shuffle"));
                        }
                        ui.label(
                            egui::RichText::new(format!("no. {}", marking.layout))
                                .small()
                                .color(ink.faint),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        for step in 1..=4_u8 {
                            let layout = marking.layout.wrapping_add(step.wrapping_mul(61));
                            let mut variant = design.clone();
                            if let Form::Sculpted { sculpt } = &mut variant.form
                                && let Some(m) = sculpt.markings.get_mut(index)
                            {
                                m.layout = layout;
                            }
                            let (texture, size) = self.pictures.variant(
                                ui.ctx(),
                                &format!("marking-{index}-layout-{layout}"),
                                &variant,
                                &self.base,
                                self.editor.changes,
                                false,
                            );
                            if picture_choice(ui, &texture, size, false, "Lay it out like this")
                                .clicked()
                            {
                                marking.layout = layout;
                                key = Some(format!("marking-{index}-try"));
                            }
                        }
                    });
                    if ui.small_button("Remove").clicked() {
                        remove = Some(index);
                    }
                });
            ui.add_space(4.0);
        }
        if let Some(index) = remove {
            markings.remove(index);
            key = Some("marking-remove".to_owned());
        }
        if markings.len() < MAX_MARKINGS
            && ui
                .button("+ Add a marking")
                .on_hover_text("Up to four, each laid over the last.")
                .clicked()
        {
            markings.push(Marking::new(MarkingKind::Spots, sculpt.coat.secondary));
            key = Some("marking-add".to_owned());
        }
        if let Some(key) = key {
            let gesture = if key.ends_with("remove") || key.ends_with("add") {
                None
            } else {
                Some(key)
            };
            self.editor.edit(gesture.as_deref(), |d| {
                if let Form::Sculpted { sculpt } = &mut d.form {
                    sculpt.markings = markings;
                }
            });
        }
    }

    /// Formiga's face layouts as pictures of `design` wearing each, `wear` putting one on it.
    /// Returns the one chosen, if one was.
    fn face_layouts(
        &mut self,
        ui: &mut egui::Ui,
        design: &Design,
        current: u8,
        wear: impl Fn(&mut Design, u8),
    ) -> Option<u8> {
        let ctx = ui.ctx().clone();
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            for template in 1..=formiga_core::FACE_TEMPLATES {
                let mut variant = design.clone();
                wear(&mut variant, template);
                let (texture, size) = self.pictures.variant(
                    &ctx,
                    &format!("face-layout-{template}"),
                    &variant,
                    &self.base,
                    self.editor.changes,
                    true,
                );
                let hover = format!("Face layout {template}");
                if picture_choice(ui, &texture, size, template == current, &hover).clicked()
                    && template != current
                {
                    chosen = Some(template);
                }
            }
        });
        chosen
    }

    fn face_page(&mut self, ui: &mut egui::Ui, design: &Design) {
        let ink = style::ink(ui.visuals().dark_mode);
        heading(ui, "Layout");
        match &design.form {
            Form::Sculpted { sculpt } => {
                let chosen = self.face_layouts(ui, design, sculpt.face_template, |d, template| {
                    if let Form::Sculpted { sculpt } = &mut d.form {
                        sculpt.face_template = template;
                    }
                });
                if let Some(template) = chosen {
                    self.editor.edit(None, |d| {
                        if let Form::Sculpted { sculpt } = &mut d.form {
                            sculpt.face_template = template;
                        }
                    });
                }
            }
            Form::Companion { recipe } => {
                let mut r = *recipe;
                if r.archetype > 0 {
                    let chosen = self.face_layouts(ui, design, r.face_template, |d, template| {
                        if let Form::Companion { recipe } = &mut d.form {
                            recipe.face_template = template;
                        }
                    });
                    if let Some(template) = chosen {
                        r.face_template = template;
                        self.editor
                            .edit(None, |d| d.form = Form::Companion { recipe: r });
                    }
                } else if choose(
                    ui,
                    "Eyes",
                    "recipe-classic-face",
                    &mut r.classic.face,
                    &[
                        (0, "Large"),
                        (1, "Mask"),
                        (2, "Visor"),
                        (3, "Beads"),
                        (4, "Tall"),
                        (5, "Square"),
                    ],
                ) {
                    self.editor
                        .edit(None, |d| d.form = Form::Companion { recipe: r });
                }
            }
            Form::Original => {
                ui.label(egui::RichText::new("Their original face layout stays.").color(ink.faint));
            }
        }
        heading(ui, "Features");
        let mut face = design.face;
        let mut key = None;
        if choose(
            ui,
            "Eye shape",
            "eye-shape",
            &mut face.eye_shape,
            &[
                (EyeShape::Round, "Round"),
                (EyeShape::Tall, "Tall"),
                (EyeShape::SoftSquare, "Soft square"),
            ],
        ) {
            key = Some("eye-shape");
        }
        if slider(ui, "Eye size", &mut face.eye_size, 1, 2) {
            key = Some("eye-size");
        }
        if slider(ui, "Eye spacing", &mut face.eye_spacing, 4, 7) {
            key = Some("eye-spacing");
        }
        if slider(ui, "Eye height", &mut face.vertical_offset, -1, 1) {
            key = Some("eye-height");
        }
        if choose(
            ui,
            "Pupils",
            "pupils",
            &mut face.pupil_style,
            &[
                (PupilStyle::Dot, "Dot"),
                (PupilStyle::Wide, "Wide"),
                (PupilStyle::Spark, "Spark"),
            ],
        ) {
            key = Some("pupils");
        }
        if choose(
            ui,
            "Shine",
            "shine",
            &mut face.highlight_style,
            &[
                (HighlightStyle::Single, "One"),
                (HighlightStyle::Double, "Two"),
                (HighlightStyle::Diagonal, "Slanted"),
            ],
        ) {
            key = Some("shine");
        }
        if choose(
            ui,
            "Brows",
            "brows",
            &mut face.brow_style,
            &[
                (BrowStyle::None, "None"),
                (BrowStyle::Soft, "Soft"),
                (BrowStyle::Bold, "Bold"),
            ],
        ) {
            key = Some("brows");
        }
        if choose(
            ui,
            "Mouth",
            "mouth",
            &mut face.mouth_style,
            &[
                (MouthStyle::Tiny, "Tiny"),
                (MouthStyle::Smile, "Smile"),
                (MouthStyle::Cat, "Cat"),
                (MouthStyle::Beak, "Beak"),
            ],
        ) {
            key = Some("mouth");
        }
        if choose(
            ui,
            "Cheeks",
            "cheeks",
            &mut face.cheek_style,
            &[
                (CheekStyle::None, "None"),
                (CheekStyle::Dots, "Dots"),
                (CheekStyle::Blush, "Blush"),
            ],
        ) {
            key = Some("cheeks");
        }
        if let Some(key) = key {
            let key = format!("face-{key}");
            self.editor.edit(Some(&key), |d| d.face = face);
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Every face keeps all of its expressions: try React, Rest and Celebrate below.",
            )
            .small()
            .color(ink.faint),
        );
    }
}
