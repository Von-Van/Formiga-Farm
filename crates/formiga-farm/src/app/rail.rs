//! The shelves along the left: where a design can start from. Picking one is a change like any
//! other, so it can be undone.

use super::{Deletable, Dialog, FarmApp, Shelf, style};
use crate::presets;
use eframe::egui;
use formiga_core::forms::Design;
use formiga_farm_contract::FarmMode;

enum Pick {
    Preset { design: Design, id: String },
    Mine { design: Design },
    Draft { id: String },
    Arrival,
}

/// What can be done with a shelf's entry besides starting from it.
enum Act {
    Compare,
    Rename(Deletable),
    Duplicate(Deletable),
    KeepAsPreset,
    CopyCode,
}

struct Entry {
    key: String,
    name: String,
    note: Option<String>,
    design: Design,
    pick: Pick,
    delete: Option<Deletable>,
}

impl FarmApp {
    pub(super) fn rail(&mut self, ui: &mut egui::Ui) {
        let ink = style::ink(ui.visuals().dark_mode);
        ui.heading("Start from");
        ui.add_space(2.0);
        if ui
            .add_sized(
                [ui.available_width(), 26.0],
                egui::Button::new("From a picture\u{2026}"),
            )
            .on_hover_text(
                "Farm reads a picture's shape, colours and markings onto one of its own bodies. You can also drop a picture on the window.",
            )
            .clicked()
        {
            self.choose_picture();
        }
        if ui
            .add_sized(
                [ui.available_width(), 26.0],
                egui::Button::new("From a design code\u{2026}"),
            )
            .on_hover_text("Paste a look someone copied from Farm with Copy code.")
            .clicked()
        {
            self.dialog = Some(Dialog::Code {
                text: String::new(),
                error: None,
            });
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for shelf in Shelf::ALL {
                if ui
                    .selectable_label(self.shelf == shelf, shelf.label())
                    .clicked()
                {
                    self.shelf = shelf;
                }
            }
        });
        ui.separator();
        let mut entries = Vec::new();
        if let FarmMode::EditExisting { creature } = &self.snapshot.mode {
            entries.push(Entry {
                key: "arrival".into(),
                name: format!("{} as they are", creature.name),
                note: Some("Their look before Farm".into()),
                design: self.editor.arrival.clone(),
                pick: Pick::Arrival,
                delete: None,
            });
        }
        match self.shelf {
            Shelf::Animals | Shelf::BodyForms => {
                let want = if self.shelf == Shelf::Animals {
                    |shelf: presets::Shelf| shelf == presets::Shelf::Animal
                } else {
                    |shelf: presets::Shelf| shelf != presets::Shelf::Animal
                };
                for preset in self.presets.iter().filter(|p| want(p.shelf)) {
                    let note = match preset.shelf {
                        presets::Shelf::Blank => Some("Nothing on it yet".to_owned()),
                        presets::Shelf::BodyForm => Some(
                            if preset.design.form.sculpt().is_some() {
                                "Sculpted body"
                            } else {
                                "Companion body"
                            }
                            .to_owned(),
                        ),
                        presets::Shelf::Animal => None,
                    };
                    entries.push(Entry {
                        key: format!("preset:{}", preset.id),
                        name: preset.name.to_owned(),
                        note,
                        design: preset.design.clone(),
                        pick: Pick::Preset {
                            design: preset.design.clone(),
                            id: preset.id.to_owned(),
                        },
                        delete: None,
                    });
                }
            }
            Shelf::Mine => {
                for preset in &self.mine {
                    entries.push(Entry {
                        key: format!(
                            "mine:{}:{}",
                            preset.id,
                            preset.saved_at_utc.unix_timestamp()
                        ),
                        name: preset.name.clone(),
                        note: None,
                        design: preset.design.clone(),
                        pick: Pick::Mine {
                            design: preset.design.clone(),
                        },
                        delete: Some(Deletable::Preset(preset.id.clone())),
                    });
                }
            }
            Shelf::Drafts => {
                for draft in &self.drafts {
                    let mut note = draft
                        .for_name
                        .as_ref()
                        .map(|name| format!("A new look for {name}"));
                    if draft.id == self.draft.id {
                        note = Some("On the workbench".into());
                    }
                    entries.push(Entry {
                        key: format!("draft:{}:{}", draft.id, draft.saved_at_utc.unix_timestamp()),
                        name: draft.name.clone(),
                        note,
                        design: draft.design.clone(),
                        pick: Pick::Draft {
                            id: draft.id.clone(),
                        },
                        delete: Some(Deletable::Draft(draft.id.clone())),
                    });
                }
            }
        }
        let empty = match self.shelf {
            Shelf::Mine => Some(
                "Nothing here yet. \u{201c}Save as preset\u{201d} keeps a design here to start from again.",
            ),
            Shelf::Drafts => {
                Some("Drafts are kept here as you work, and come back after Farm closes.")
            }
            _ => None,
        };
        // A full shelf says so before it has to let a draft go: the next new draft takes the
        // oldest one's place.
        let full = self.shelf == Shelf::Drafts
            && self.drafts.len() >= crate::store::MAX_DRAFTS
            && self.drafts.iter().all(|draft| draft.id != self.draft.id);
        if full {
            ui.label(
                egui::RichText::new(format!(
                    "The shelf is full at {} drafts. The next one kept lets the oldest go, so delete any you don\u{2019}t need, or save one as a preset to keep it for good.",
                    crate::store::MAX_DRAFTS
                ))
                .small()
                .color(ink.warn),
            );
        }
        let mut picked = None;
        let mut delete = None;
        let mut act = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if entries.len()
                    <= usize::from(matches!(self.snapshot.mode, FarmMode::EditExisting { .. }))
                    && let Some(empty) = empty
                {
                    ui.label(egui::RichText::new(empty).color(ink.faint));
                }
                for (index, entry) in entries.iter().enumerate() {
                    let (texture, size) =
                        self.pictures
                            .thumb(ui.ctx(), &entry.key, &entry.design, &self.base);
                    let contents = |ui: &mut egui::Ui| {
                        let scale = (40.0 / size.y.max(size.x * 0.8)).floor().clamp(1.0, 3.0);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(52.0, 44.0), egui::Sense::hover());
                        let image = egui::Rect::from_center_size(rect.center(), size * scale);
                        ui.painter().image(
                            texture.id(),
                            image,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&entry.name).strong());
                            if let Some(note) = &entry.note {
                                ui.label(egui::RichText::new(note).small().color(ink.faint));
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(what) = &entry.delete
                                && ui
                                    .small_button("\u{2715}")
                                    .on_hover_text("Throw this away. Nobody in the colony changes.")
                                    .clicked()
                            {
                                delete = Some((what.clone(), entry.name.clone()));
                            }
                            ui.menu_button("\u{2026}", |ui| {
                                if let Some(chosen) = entry_menu(ui, entry) {
                                    act = Some((index, chosen));
                                    ui.close();
                                }
                            })
                            .response
                            .on_hover_text("Compare, rename, copy and more");
                        });
                    };
                    // The whole row is one button: it takes keyboard focus, Enter or Space picks
                    // it, and a screen reader hears its name and what it is. It sits behind the
                    // delete and menu buttons, so those still take their own clicks.
                    let row = egui::UiBuilder::new()
                        .id_salt(("rail-entry", index))
                        .layout(egui::Layout::left_to_right(egui::Align::Center))
                        .sense(egui::Sense::click());
                    let response = ui
                        .horizontal(|ui| ui.scope_builder(row, contents).response)
                        .inner;
                    let spoken = match &entry.note {
                        Some(note) => format!("Start from {}: {note}", entry.name),
                        None => format!("Start from {}", entry.name),
                    };
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &spoken)
                    });
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            response.rect.expand(2.0),
                            6.0,
                            ui.visuals().selection.stroke,
                            egui::StrokeKind::Outside,
                        );
                    } else if response.hovered() {
                        ui.painter().rect_stroke(
                            response.rect.expand(2.0),
                            6.0,
                            egui::Stroke::new(1.0, ink.chosen),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if response.clicked() {
                        picked = Some(index);
                    }
                    response.context_menu(|ui| {
                        if let Some(chosen) = entry_menu(ui, entry) {
                            act = Some((index, chosen));
                            ui.close();
                        }
                    });
                    ui.add_space(2.0);
                }
            });
        if let Some((what, name)) = delete {
            self.dialog = Some(Dialog::Delete { what, name });
        } else if let Some((index, act)) = act {
            let entry = entries.swap_remove(index);
            self.act_on(ui.ctx(), entry, act);
        } else if let Some(index) = picked {
            let entry = entries.swap_remove(index);
            match entry.pick {
                Pick::Preset { design, id } => self.editor.start_from(design, Some(id)),
                Pick::Mine { design, .. } => self.editor.start_from(design, None),
                Pick::Arrival => self.editor.restore_arrival(),
                Pick::Draft { id } => {
                    if let Some(draft) = self.drafts.iter().find(|d| d.id == id).cloned() {
                        // Keep the one on the bench first, then carry on with this one.
                        self.keep_draft();
                        self.editor
                            .start_from(draft.design.clone(), draft.preset.clone());
                        self.draft = draft;
                        self.kept_changes = self.editor.changes;
                    }
                }
            }
        }
    }
}

impl FarmApp {
    fn act_on(&mut self, ctx: &egui::Context, entry: Entry, act: Act) {
        match act {
            Act::Compare => self.compare(&entry.name, entry.design),
            Act::Rename(what) => {
                self.dialog = Some(Dialog::Rename {
                    what,
                    name: entry.name,
                });
            }
            Act::Duplicate(Deletable::Draft(id)) => {
                if let Some(draft) = self.drafts.iter().find(|d| d.id == id) {
                    let mut copy = crate::store::Draft::new(
                        &format!("{} (copy)", draft.name),
                        draft.design.clone(),
                        draft.preset.clone(),
                        draft.for_name.clone(),
                    );
                    match self.store.save_draft(&mut copy, &mut self.drafts) {
                        Ok(let_go) => {
                            if !self.say_let_go(&let_go) {
                                self.say(
                                    format!("Kept a copy as \u{201c}{}\u{201d}.", copy.name),
                                    false,
                                );
                            }
                        }
                        Err(error) => self.say(format!("Could not copy it: {error}"), true),
                    }
                }
            }
            Act::Duplicate(Deletable::Preset(_)) => {
                let copy = crate::store::PersonalPreset::new(
                    &format!("{} (copy)", entry.name),
                    entry.design,
                );
                match self.store.save_preset(&copy) {
                    Ok(()) => {
                        self.mine = self.store.presets();
                        self.say(
                            format!("Kept a copy as \u{201c}{}\u{201d}.", copy.name),
                            false,
                        );
                    }
                    Err(error) => self.say(format!("Could not copy it: {error}"), true),
                }
            }
            Act::KeepAsPreset => {
                let preset = crate::store::PersonalPreset::new(&entry.name, entry.design);
                match self.store.save_preset(&preset) {
                    Ok(()) => {
                        self.mine = self.store.presets();
                        self.say(
                            format!("Kept \u{201c}{}\u{201d} in My presets.", preset.name),
                            false,
                        );
                    }
                    Err(error) => self.say(format!("Could not keep it: {error}"), true),
                }
            }
            Act::CopyCode => self.copy_code_of(ctx, &entry.design, &entry.name),
        }
    }
}

/// The menu for one entry on a shelf. Returns what was chosen, if anything was.
fn entry_menu(ui: &mut egui::Ui, entry: &Entry) -> Option<Act> {
    let mut chosen = None;
    if ui
        .button("Compare with the workbench")
        .on_hover_text("Show it beside the design on the workbench.")
        .clicked()
    {
        chosen = Some(Act::Compare);
    }
    if let Some(what) = &entry.delete {
        if ui.button("Rename\u{2026}").clicked() {
            chosen = Some(Act::Rename(what.clone()));
        }
        if ui.button("Duplicate").clicked() {
            chosen = Some(Act::Duplicate(what.clone()));
        }
        if matches!(what, Deletable::Draft(_)) && ui.button("Keep as a preset").clicked() {
            chosen = Some(Act::KeepAsPreset);
        }
    }
    if crate::share::shareable(&entry.design)
        && ui
            .button("Copy code")
            .on_hover_text("Copy this look as a design code, to send to someone or keep.")
            .clicked()
    {
        chosen = Some(Act::CopyCode);
    }
    chosen
}
