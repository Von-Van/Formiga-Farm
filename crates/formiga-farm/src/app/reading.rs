//! "From a picture": choosing or dropping a picture, reading it out of the way of the window,
//! and offering what it was read as. Starting from one of those is a change like any other, so
//! it can be undone.

use super::{Dialog, FarmApp, style};
use crate::import::{self, Take};
use eframe::egui;
use formiga_forms::Design;
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::mpsc;

/// What reading a picture comes to: the takes, and the picture itself, small; or why not.
type Read = Result<(Vec<Take>, Option<RgbaImage>), String>;

/// A picture being read.
pub(super) struct Reading {
    name: String,
    answer: mpsc::Receiver<Read>,
}

/// The kinds of picture Farm reads, by their names' endings.
const KINDS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

fn readable(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| KINDS.iter().any(|kind| kind.eq_ignore_ascii_case(ext)))
}

impl FarmApp {
    /// Ask for a picture, and read it.
    pub(super) fn choose_picture(&mut self) {
        let chosen = rfd::FileDialog::new()
            .set_title("Start from a picture")
            .add_filter("Pictures", &KINDS)
            .pick_file();
        if let Some(path) = chosen {
            self.read_picture(path);
        }
    }

    /// Read a picture dropped on the window, if one was.
    pub(super) fn dropped_pictures(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if dropped.is_empty() || self.dialog.is_some() {
            return;
        }
        match dropped.into_iter().find(|path| readable(path)) {
            Some(path) => self.read_picture(path),
            None => self.say("Farm reads PNG, JPEG, WebP and GIF pictures.", true),
        }
    }

    /// Start reading the picture at `path`, out of the way of the window.
    pub fn read_picture(&mut self, path: PathBuf) {
        let name = path.file_name().map_or_else(
            || "the picture".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let base = self.base.clone();
        let (tell, answer) = mpsc::channel();
        std::thread::spawn(move || {
            let read = import::read_file(&path, &base).map_err(|error| error.to_string());
            // The picture itself, small, to show beside what it was read as.
            let picture = read.is_ok().then(|| image::open(&path).ok()).flatten();
            let picture = picture.map(|picture| picture.thumbnail(192, 192).to_rgba8());
            let _ = tell.send(read.map(|takes| (takes, picture)));
        });
        self.reading = Some(Reading {
            name: name.clone(),
            answer,
        });
        self.dialog = Some(Dialog::Reading { name });
    }

    /// Whether the picture being read has been, and if so, offer what it was read as.
    pub(super) fn heard_reading(&mut self, ctx: &egui::Context) {
        let Some(reading) = &self.reading else {
            return;
        };
        let read = match reading.answer.try_recv() {
            Ok(read) => read,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("reading it stopped short".to_owned()),
        };
        let name = reading.name.clone();
        self.reading = None;
        // Only if the reading is still being waited for: it may have been put aside.
        if !matches!(self.dialog, Some(Dialog::Reading { .. })) {
            return;
        }
        match read {
            Ok((takes, picture)) if !takes.is_empty() => {
                let picture = picture.map(|picture| {
                    let size = [picture.width() as usize, picture.height() as usize];
                    ctx.load_texture(
                        "farm-picture",
                        egui::ColorImage::from_rgba_unmultiplied(size, picture.as_raw()),
                        egui::TextureOptions::LINEAR,
                    )
                });
                self.dialog = Some(Dialog::Takes {
                    name,
                    takes,
                    picture,
                });
            }
            Ok(_) => {
                self.dialog = None;
                self.say(
                    format!("Could not read {name}: nothing in it to read."),
                    true,
                );
            }
            Err(error) => {
                self.dialog = None;
                self.say(format!("Could not read {name}: {error}."), true);
            }
        }
    }

    /// The window shown while a picture is read. Returns whether it stays open.
    pub(super) fn reading_dialog(&mut self, ui: &mut egui::Ui, name: &str) -> bool {
        let ink = style::ink(ui.visuals().dark_mode);
        ui.heading("Reading your picture\u{2026}");
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(egui::RichText::new(name).color(ink.faint));
        });
        ui.label("Farm is looking for its shape, its colours and its markings.");
        ui.add_space(6.0);
        if ui.button("Never mind").clicked() {
            self.reading = None;
            return false;
        }
        true
    }

    /// The window offering what a picture was read as. Returns whether it stays open.
    pub(super) fn takes_dialog(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        takes: &[Take],
        picture: Option<&egui::TextureHandle>,
    ) -> bool {
        let ink = style::ink(ui.visuals().dark_mode);
        ui.set_max_width(560.0);
        ui.heading(format!("Formigas from {name}"));
        ui.label(
            "Farm read the picture's shape, colours and markings onto its own bodies. Pick one to start from: everything can be changed after, and Undo brings back what you had.",
        );
        ui.add_space(8.0);
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            if let Some(picture) = picture {
                let size = picture.size_vec2();
                let scale = (118.0 / size.x.max(size.y)).min(1.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.image((picture.id(), size * scale));
                    ui.label(egui::RichText::new("Your picture").small().color(ink.faint));
                });
            }
            for (index, take) in takes.iter().enumerate() {
                let key = format!("take:{}", take.design.revision());
                let (texture, size) = self
                    .pictures
                    .thumb(ui.ctx(), &key, &take.design, &self.base);
                let card = egui::Frame::new()
                    .fill(ink.card)
                    .stroke(egui::Stroke::new(1.0, ink.line))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(118.0);
                        ui.vertical_centered(|ui| {
                            let scale = (96.0 / size.x.max(size.y)).clamp(1.0, 3.0);
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(110.0, 96.0), egui::Sense::hover());
                            ui.painter().image(
                                texture.id(),
                                egui::Rect::from_center_size(rect.center(), size * scale),
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                            ui.label(egui::RichText::new(take.title).strong());
                            ui.label(egui::RichText::new(&take.summary).small().color(ink.faint));
                        });
                    });
                let response = ui.interact(
                    card.response.rect,
                    ui.id().with(("take", index)),
                    egui::Sense::click(),
                );
                if response.hovered() {
                    ui.painter().rect_stroke(
                        card.response.rect,
                        8.0,
                        egui::Stroke::new(1.5, ink.chosen),
                        egui::StrokeKind::Inside,
                    );
                }
                if response.on_hover_text("Start from this one").clicked() {
                    chosen = Some(index);
                }
            }
        });
        ui.add_space(8.0);
        if let Some(index) = chosen {
            // The picture gives the body and coat; the face stays as it is on the workbench.
            let design = Design {
                form: takes[index].design.form.clone(),
                face: self.editor.design().face,
            };
            self.editor.start_from(design, None);
            return false;
        }
        !ui.button("Keep what I have").clicked()
    }
}
