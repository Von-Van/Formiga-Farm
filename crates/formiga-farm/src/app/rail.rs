//! The shelves along the left: where a design can start from. Picking one is a change like any
//! other, so it can be undone.

use super::{Deletable, Dialog, FarmApp, Shelf, style};
use crate::presets;
use eframe::egui;
use formiga_farm_contract::FarmMode;
use formiga_forms::Design;

enum Pick {
    Preset { design: Design, id: String },
    Mine { design: Design },
    Draft { id: String },
    Arrival,
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
        let mut picked = None;
        let mut delete = None;
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
                    let row = ui.horizontal(|ui| {
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
                        if let Some(what) = &entry.delete {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button("\u{2715}")
                                        .on_hover_text(
                                            "Throw this away. Nobody in the colony changes.",
                                        )
                                        .clicked()
                                    {
                                        delete = Some((what.clone(), entry.name.clone()));
                                    }
                                },
                            );
                        }
                    });
                    let response = ui.interact(
                        row.response.rect,
                        ui.id().with(("rail-entry", index)),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter().rect_stroke(
                            row.response.rect.expand(2.0),
                            6.0,
                            egui::Stroke::new(1.0, ink.chosen),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if response.clicked() {
                        picked = Some(index);
                    }
                    ui.add_space(2.0);
                }
            });
        if let Some((what, name)) = delete {
            self.dialog = Some(Dialog::Delete { what, name });
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
