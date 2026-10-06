//! The workshop window: shelves of starting points on the left, the creature on its stage in the
//! middle, the inspector for whichever layer is being worked on to the right, and a strip along
//! the bottom for how it moves and how it looks in each of the other apps. All of it over the ant
//! farm.

mod inspector;
mod pictures;
mod rail;
mod stage;
pub mod style;

use crate::editor::{Editor, Layer};
use crate::habitat::{Daylight, Habitat, PIXEL};
use crate::notices::{self, Notice};
use crate::presets::Preset;
use crate::session::{Heard, Host};
use crate::store::{Draft, PersonalPreset, Store, WindowPlace};
use eframe::egui::{self, Color32};
use formiga_core::AppearanceGenome;
use formiga_farm_contract::{FarmMode, FarmSnapshot, Lineage, ProposalKind, Verdict};
use formiga_forms::{Design, Intent};
use pictures::Pictures;
use std::path::PathBuf;

/// How long a short message stays up, in seconds.
const TOAST_SECS: f64 = 6.0;

/// Below this width the shelves and the inspector become drawers, one open at a time.
const NARROW: f32 = 980.0;

/// Which shelf of starting points the rail shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Animals,
    BodyForms,
    Mine,
    Drafts,
}

impl Shelf {
    const ALL: [Self; 4] = [Self::Animals, Self::BodyForms, Self::Mine, Self::Drafts];

    fn label(self) -> &'static str {
        match self {
            Self::Animals => "Animals",
            Self::BodyForms => "Body forms",
            Self::Mine => "My presets",
            Self::Drafts => "Drafts",
        }
    }
}

/// Where the design is shown on the stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// Large, on the workbench.
    Stage,
    /// At its own size on a desktop, as it will live there.
    Desktop,
    /// Stood on a floor in Formiga Home.
    Home,
    /// Out on the grass of Formiga Hill.
    Hill,
}

impl View {
    const ALL: [Self; 4] = [Self::Stage, Self::Desktop, Self::Home, Self::Hill];

    fn label(self) -> &'static str {
        match self {
            Self::Stage => "Workbench",
            Self::Desktop => "Desktop",
            Self::Home => "Home",
            Self::Hill => "Hill",
        }
    }
}

enum Dialog {
    /// The creature's look changed in Desktop after the session opened.
    Stale { current: Design, revision: String },
    /// Naming a personal preset.
    SavePreset { name: String },
    /// Making sure before a draft or a preset is thrown away.
    Delete { what: Deletable, name: String },
    /// Desktop ended the session.
    Recalled,
}

#[derive(Clone)]
enum Deletable {
    Draft(String),
    Preset(String),
}

struct Toast {
    text: String,
    at: f64,
    trouble: bool,
}

pub struct FarmApp {
    snapshot: FarmSnapshot,
    base: AppearanceGenome,
    host: Host,
    store: Store,
    _open: Option<crate::store::Open>,
    editor: Editor,
    draft: Draft,
    /// The editor's change count when the draft was last kept.
    kept_changes: u64,
    /// When the design last changed, so a draft is kept once the owner pauses.
    changed_at: f64,
    seen_changes: u64,
    layer: Layer,
    shelf: Shelf,
    intent: Intent,
    view: View,
    facing_right: bool,
    before: bool,
    grid: bool,
    seed: u32,
    pending: Option<(u32, f64)>,
    expected_revision: Option<String>,
    dialog: Option<Dialog>,
    toast: Option<Toast>,
    pictures: Pictures,
    habitat: Option<(Habitat, egui::TextureHandle, [u32; 2])>,
    presets: Vec<Preset>,
    drafts: Vec<Draft>,
    mine: Vec<PersonalPreset>,
    notices: (u64, Vec<Notice>),
    clock: f64,
    rail_open: bool,
    inspector_open: bool,
    place: Option<WindowPlace>,
    snap: Option<(PathBuf, f64, bool)>,
    left: bool,
}

/// What the session opens on.
pub struct Opening {
    pub snapshot: FarmSnapshot,
    pub host: Host,
    pub store: Store,
    pub open: Option<crate::store::Open>,
    /// A draft or preset to open on instead of the creature or a blank.
    pub start: Option<(Design, Option<String>, String)>,
}

impl FarmApp {
    pub fn new(ctx: &egui::Context, opening: Opening) -> anyhow::Result<Self> {
        let Opening {
            snapshot,
            host,
            store,
            open,
            start,
        } = opening;
        let base = snapshot.base_genome()?;
        let text_scale =
            f32::from(snapshot.presentation.text_scale_percent.clamp(100, 150)) / 100.0;
        style::apply(ctx, text_scale);
        match snapshot.presentation.theme {
            formiga_travel::Theme::Light => ctx.set_theme(egui::ThemePreference::Light),
            formiga_travel::Theme::Dark => ctx.set_theme(egui::ThemePreference::Dark),
            _ => ctx.set_theme(egui::ThemePreference::System),
        }
        let (arrival, preset, name, for_name, expected_revision) = match &snapshot.mode {
            FarmMode::EditExisting { creature } => (
                creature.design()?,
                None,
                format!("{}'s new look", creature.name),
                Some(creature.name.clone()),
                Some(creature.revision.clone()),
            ),
            FarmMode::Create { .. } => (
                crate::presets::blank(),
                Some("blank".to_owned()),
                "New Formiga".to_owned(),
                None,
                None,
            ),
        };
        let mut editor = Editor::new(arrival.clone(), preset.clone());
        let mut draft = Draft::new(&name, arrival, preset, for_name);
        if let Some((design, preset, name)) = start {
            editor.start_from(design.clone(), preset.clone());
            draft = Draft::new(&name, design, preset, draft.for_name.clone());
        }
        let place = store.data().and_then(WindowPlace::load);
        let drafts = store.drafts();
        let mine = store.presets();
        let shelf = if matches!(snapshot.mode, FarmMode::EditExisting { .. }) {
            Shelf::BodyForms
        } else {
            Shelf::Animals
        };
        Ok(Self {
            snapshot,
            base,
            host,
            store,
            _open: open,
            kept_changes: editor.changes,
            seen_changes: editor.changes,
            editor,
            draft,
            changed_at: 0.0,
            layer: Layer::Form,
            shelf,
            intent: Intent::Idle,
            view: View::Stage,
            facing_right: true,
            before: false,
            grid: false,
            seed: 1,
            pending: None,
            expected_revision,
            dialog: None,
            toast: None,
            pictures: Pictures::default(),
            habitat: None,
            presets: crate::presets::all(),
            drafts,
            mine,
            notices: (u64::MAX, Vec::new()),
            clock: 0.0,
            rail_open: true,
            inspector_open: true,
            place,
            snap: None,
            left: false,
        })
    }

    /// For review only: after `at` seconds save a picture of the window to `path`, and close.
    pub fn snap(&mut self, path: PathBuf, at: f64) {
        self.snap = Some((path, at, false));
    }

    /// For review only: open on a layer, a pose or a view.
    pub fn show(&mut self, layer: Option<Layer>, intent: Option<Intent>, view: Option<View>) {
        if let Some(layer) = layer {
            self.layer = layer;
        }
        if let Some(intent) = intent {
            self.intent = intent;
        }
        if let Some(view) = view {
            self.view = view;
        }
    }

    pub fn shelf(&mut self, shelf: Shelf) {
        self.shelf = shelf;
    }

    fn reduce_motion(&self) -> bool {
        self.snapshot.presentation.reduce_motion
    }

    fn say(&mut self, text: impl Into<String>, trouble: bool) {
        self.toast = Some(Toast {
            text: text.into(),
            at: self.clock,
            trouble,
        });
    }

    fn creature_name(&self) -> Option<&str> {
        self.snapshot
            .creature()
            .map(|creature| creature.name.as_str())
    }

    /// The design shown: the one on the workbench, or how it was when the session opened.
    fn shown(&self) -> &Design {
        if self.before {
            &self.editor.arrival
        } else {
            self.editor.design()
        }
    }

    fn notices(&mut self) -> &[Notice] {
        if self.notices.0 != self.editor.changes {
            let found = notices::check(
                self.editor.design(),
                &self.snapshot,
                &self.base,
                self.editor.set_aside(),
            );
            self.notices = (self.editor.changes, found);
        }
        &self.notices.1
    }

    /// Keep the draft, now.
    fn keep_draft(&mut self) {
        if self.editor.changes == self.kept_changes {
            return;
        }
        self.draft.design = self.editor.design().clone();
        self.draft.preset = self.editor.preset.clone();
        match self.store.save_draft(&mut self.draft) {
            Ok(()) => {
                self.kept_changes = self.editor.changes;
                self.drafts = self.store.drafts();
            }
            Err(error) => self.say(format!("The draft could not be kept: {error}"), true),
        }
    }

    /// Keep the draft once the owner has paused, so a crash loses nothing.
    fn keep_draft_when_paused(&mut self) {
        if self.editor.changes != self.seen_changes {
            self.seen_changes = self.editor.changes;
            self.changed_at = self.clock;
        }
        if self.editor.changes != self.kept_changes && self.clock - self.changed_at > 0.8 {
            self.keep_draft();
        }
    }

    fn apply(&mut self) {
        if self.pending.is_some() || notices::blocked(self.notices()) {
            return;
        }
        let kind = match &self.snapshot.mode {
            FarmMode::EditExisting { creature } => ProposalKind::EditExisting {
                target: creature.id,
                expected_revision: self
                    .expected_revision
                    .clone()
                    .unwrap_or_else(|| creature.revision.clone()),
            },
            FarmMode::Create { .. } => ProposalKind::CreateNew,
        };
        self.keep_draft();
        let lineage = Lineage::new(self.editor.preset.as_deref(), Some(&self.draft.name));
        let design = self.editor.design().clone();
        match self.host.propose(&self.snapshot, kind, &design, lineage) {
            Ok(serial) => self.pending = Some((serial, self.clock)),
            Err(error) => self.say(format!("{error:#}"), true),
        }
    }

    fn listen(&mut self, ctx: &egui::Context) {
        let serial = self.pending.map(|(serial, _)| serial);
        match self.host.listen(serial) {
            Heard::Nothing => {}
            Heard::Recalled => {
                self.keep_draft();
                self.dialog = Some(Dialog::Recalled);
                self.pending = None;
            }
            Heard::Verdict(verdict) => {
                self.pending = None;
                let name = self.creature_name().unwrap_or("Your Formiga").to_owned();
                match verdict {
                    Verdict::Kept { revision } => {
                        self.expected_revision = Some(revision);
                        self.editor.arrival = self.editor.design().clone();
                        self.pictures.forget_before();
                        self.say(format!("{name} has their new look."), false);
                    }
                    Verdict::Welcomed => self.say(
                        "Desktop is welcoming your new Formiga. Give them a name in Desktop's Journal.",
                        false,
                    ),
                    Verdict::Stale { current, revision } => {
                        self.dialog = Some(Dialog::Stale { current, revision });
                    }
                    Verdict::Unavailable => self.say(
                        format!("{name} is out with another Formiga app. Nothing changed; apply again when they are back."),
                        true,
                    ),
                    Verdict::Gone => self.say(
                        format!("{name} is no longer in the colony. Your design is kept as a draft."),
                        true,
                    ),
                    Verdict::Unsupported => self.say(
                        "Desktop cannot keep this design. It is kept here as a draft.",
                        true,
                    ),
                    Verdict::Invalid | Verdict::Unknown => self.say(
                        "Desktop could not use that design. Nothing changed; it is kept here as a draft.",
                        true,
                    ),
                }
                ctx.request_repaint();
            }
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || ctx.egui_wants_keyboard_input() {
            return;
        }
        let command = egui::Modifiers::COMMAND;
        let shift_command = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
        ctx.input_mut(|input| {
            if input.consume_key(shift_command, egui::Key::Z) {
                self.editor.redo();
            } else if input.consume_key(command, egui::Key::Z) {
                self.editor.undo();
            } else if input.consume_key(command, egui::Key::Y) {
                self.editor.redo();
            }
            if input.consume_key(command, egui::Key::S) {
                self.kept_changes = u64::MAX;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::B) {
                self.before = !self.before;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::F) {
                self.facing_right = !self.facing_right;
            }
            for (key, intent) in [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
            ]
            .into_iter()
            .zip(Intent::ALL)
            {
                if input.consume_key(egui::Modifiers::NONE, key) {
                    self.intent = intent;
                }
            }
        });
        if self.kept_changes == u64::MAX {
            // Asked to keep the draft now, whatever the count says.
            self.kept_changes = self.editor.changes.wrapping_sub(1);
            self.keep_draft();
            self.say("Draft kept.", false);
        }
    }

    fn take_snap(&mut self, ctx: &egui::Context) {
        let Some((path, at, asked)) = self.snap.clone() else {
            return;
        };
        if !asked && self.clock >= at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.snap = Some((path.clone(), at, true));
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let mut canvas = formiga_art::Canvas::new(width as u32, height as u32);
            for (index, pixel) in image.pixels.iter().enumerate() {
                let [r, g, b, a] = pixel.to_srgba_unmultiplied();
                canvas.set(
                    (index % width) as i32,
                    (index / width) as i32,
                    formiga_art::Rgba::new(r, g, b, a),
                );
            }
            if let Err(error) = crate::review::write_png(&canvas, &path) {
                eprintln!("formiga-farm: {error:#}");
            }
            self.snap = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// The habitat behind everything, drawn again only when the window's size changes.
    fn habitat(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let size = [
            (screen.width() / PIXEL).ceil().max(64.0) as u32,
            (screen.height() / PIXEL).ceil().max(64.0) as u32,
        ];
        let hour = time::OffsetDateTime::now_local()
            .map(|now| now.hour())
            .unwrap_or(12);
        let daylight = if self.snap.is_some() {
            Daylight::Day
        } else {
            Daylight::at_hour(hour)
        };
        let stale = self
            .habitat
            .as_ref()
            .is_none_or(|(habitat, _, at)| *at != size || habitat.daylight != daylight);
        if stale {
            let habitat = Habitat::new(size[0], size[1], 11, daylight);
            let texture = pictures::texture(ctx, "habitat", &habitat.canvas);
            self.habitat = Some((habitat, texture, size));
        }
        let Some((habitat, texture, size)) = &self.habitat else {
            return;
        };
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect = egui::Rect::from_min_size(
            screen.min,
            egui::vec2(size[0] as f32 * PIXEL, size[1] as f32 * PIXEL),
        );
        painter.image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        // The ants, a pixel and a half long, slow and dark: never brighter than the soil.
        let seconds = if self.reduce_motion() {
            0.0
        } else {
            self.clock as f32
        };
        for (x, y, right) in habitat.ants(seconds) {
            let at = screen.min + egui::vec2(x * PIXEL, y * PIXEL);
            let ink = Color32::from_rgb(0x2a, 0x1d, 0x18);
            let head = if right { PIXEL } else { -PIXEL };
            painter.rect_filled(
                egui::Rect::from_min_size(at, egui::vec2(PIXEL, PIXEL)),
                0.0,
                ink,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(at + egui::vec2(head, 0.0), egui::vec2(PIXEL, PIXEL)),
                0.0,
                ink,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(
                    at - egui::vec2(head, -PIXEL * 0.5),
                    egui::vec2(PIXEL, PIXEL * 0.5),
                ),
                0.0,
                ink,
            );
        }
    }

    fn leave(&mut self) {
        if self.left {
            return;
        }
        self.left = true;
        self.keep_draft();
        if let (Some(place), Some(data)) = (self.place, self.store.data()) {
            place.save(data);
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, narrow: bool) {
        let ink = style::ink(ui.visuals().dark_mode);
        ui.horizontal(|ui| {
            if ui
                .button("Back to Journal")
                .on_hover_text("Close Farm and go back to Desktop's Journal. Your draft is kept.")
                .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if narrow {
                if ui.selectable_label(self.rail_open, "Shelves").clicked() {
                    self.rail_open = !self.rail_open;
                    self.inspector_open = false;
                }
                if ui.selectable_label(self.inspector_open, "Inspector").clicked() {
                    self.inspector_open = !self.inspector_open;
                    self.rail_open = false;
                }
            }
            ui.separator();
            let mut name = self.draft.name.clone();
            let edit = ui.add(
                egui::TextEdit::singleline(&mut name)
                    .desired_width(180.0)
                    .char_limit(crate::store::MAX_NAME_CHARS)
                    .hint_text("Name this draft"),
            );
            if edit.changed() {
                self.draft.name = crate::store::clean_name(&name, "Untitled");
                self.kept_changes = self.kept_changes.wrapping_sub(1);
                self.changed_at = self.clock;
            }
            ui.separator();
            if ui
                .add_enabled(self.editor.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.editor.undo();
            }
            if ui
                .add_enabled(self.editor.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.editor.redo();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let blocked = notices::blocked(self.notices());
                let label = match &self.snapshot.mode {
                    FarmMode::EditExisting { creature } => format!("Apply to {}", creature.name),
                    FarmMode::Create { .. } => "Add to colony".to_owned(),
                };
                let waiting = self.pending.is_some();
                let apply = egui::Button::new(egui::RichText::new(label).strong())
                    .fill(if blocked { ink.card } else { ink.chosen.gamma_multiply(0.35) });
                let response = ui.add_enabled(!blocked && !waiting, apply);
                let response = if blocked {
                    response.on_disabled_hover_text("See the notes under the stage for why.")
                } else {
                    response.on_hover_text(match &self.snapshot.mode {
                        FarmMode::EditExisting { .. } => {
                            "Hand this design to Desktop. It changes only how they look; who they are stays the same."
                        }
                        FarmMode::Create { .. } => {
                            "Hand this design to Desktop's welcome, where you give your new Formiga a name."
                        }
                    })
                };
                if response.clicked() {
                    self.apply();
                }
                if ui
                    .button("Save as preset\u{2026}")
                    .on_hover_text("Keep this design on your own shelf, to start from again.")
                    .clicked()
                {
                    self.dialog = Some(Dialog::SavePreset {
                        name: self.draft.name.clone(),
                    });
                }
                if ui
                    .button("Keep draft")
                    .on_hover_text("Drafts are kept as you go, too. \u{2318}S / Ctrl+S")
                    .clicked()
                {
                    self.kept_changes = self.editor.changes.wrapping_sub(1);
                    self.keep_draft();
                    self.say("Draft kept.", false);
                }
                if let Some(label) = self.host.label() {
                    ui.label(egui::RichText::new(label).small().color(ink.faint));
                }
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let name = self.creature_name().unwrap_or("This Formiga").to_owned();
        let mut keep_open = true;
        let modal = egui::Modal::new(egui::Id::new("farm-dialog")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            match &dialog {
                Dialog::Stale { current, revision } => {
                    ui.heading(format!("{name}'s look changed in Desktop"));
                    ui.label(
                        "It changed after Farm opened, so nothing was applied. You can start again from how they look now, compare yours against it, or keep yours as a preset.",
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Reopen their look now").clicked() {
                            self.editor.arrival = current.clone();
                            self.editor.start_from(current.clone(), None);
                            self.expected_revision = Some(revision.clone());
                            self.pictures.forget_before();
                            keep_open = false;
                        }
                        if ui
                            .button("Compare with mine")
                            .on_hover_text("Keep your design, and show theirs as \u{201c}before\u{201d}.")
                            .clicked()
                        {
                            self.editor.arrival = current.clone();
                            self.expected_revision = Some(revision.clone());
                            self.pictures.forget_before();
                            self.before = true;
                            keep_open = false;
                        }
                        if ui.button("Keep mine as a preset").clicked() {
                            let preset =
                                PersonalPreset::new(&self.draft.name, self.editor.design().clone());
                            match self.store.save_preset(&preset) {
                                Ok(()) => {
                                    self.mine = self.store.presets();
                                    self.say(format!("Kept \u{201c}{}\u{201d} in My presets.", preset.name), false);
                                }
                                Err(error) => self.say(format!("Could not keep it: {error}"), true),
                            }
                            self.editor.arrival = current.clone();
                            self.expected_revision = Some(revision.clone());
                            self.pictures.forget_before();
                            keep_open = false;
                        }
                    });
                }
                Dialog::SavePreset { name } => {
                    ui.heading("Save as a preset");
                    ui.label("A preset is only a look to start from. It never changes anyone in the colony.");
                    let mut text = name.clone();
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .char_limit(crate::store::MAX_NAME_CHARS)
                            .hint_text("Its name"),
                    );
                    edit.request_focus();
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() || enter {
                            let preset = PersonalPreset::new(&text, self.editor.design().clone());
                            match self.store.save_preset(&preset) {
                                Ok(()) => {
                                    self.mine = self.store.presets();
                                    self.shelf = Shelf::Mine;
                                    self.say(format!("Kept \u{201c}{}\u{201d} in My presets.", preset.name), false);
                                }
                                Err(error) => self.say(format!("Could not keep it: {error}"), true),
                            }
                            keep_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            keep_open = false;
                        }
                    });
                    if keep_open {
                        self.dialog = Some(Dialog::SavePreset { name: text });
                    }
                }
                Dialog::Delete { what, name } => {
                    ui.heading(format!("Throw away \u{201c}{name}\u{201d}?"));
                    ui.label("Only this design goes. Nobody in the colony changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Throw it away").clicked() {
                            match what {
                                Deletable::Draft(id) => {
                                    self.store.delete_draft(id);
                                    self.drafts = self.store.drafts();
                                    if *id == self.draft.id {
                                        // The design on the bench is kept as a fresh draft.
                                        self.draft.id = crate::store::new_id();
                                        self.kept_changes = self.kept_changes.wrapping_sub(1);
                                    }
                                }
                                Deletable::Preset(id) => {
                                    self.store.delete_preset(id);
                                    self.mine = self.store.presets();
                                }
                            }
                            keep_open = false;
                        }
                        if ui.button("Keep it").clicked() {
                            keep_open = false;
                        }
                    });
                }
                Dialog::Recalled => {
                    ui.heading("Desktop has closed this session");
                    ui.label("Nothing more will be applied. Your design is kept in Drafts for next time.");
                    if ui.button("Close Farm").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        });
        if modal.should_close() && !matches!(dialog, Dialog::Recalled) {
            keep_open = false;
        }
        if keep_open && self.dialog.is_none() {
            self.dialog = Some(dialog);
        }
    }

    fn toast(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if self
            .toast
            .as_ref()
            .is_some_and(|toast| self.clock - toast.at > TOAST_SECS)
        {
            self.toast = None;
        }
        let waiting = self.pending.map(|(_, since)| self.clock - since);
        let ink = style::ink(ui.visuals().dark_mode);
        let (text, color) = match (&self.toast, waiting) {
            (_, Some(waited)) if waited > 4.0 && !self.host.is_rehearsal() => (
                "Desktop has the design. If it is busy, it will look at it when Farm closes."
                    .to_owned(),
                ink.faint,
            ),
            (_, Some(_)) => (
                "Handing the design to Desktop\u{2026}".to_owned(),
                ink.faint,
            ),
            (Some(toast), None) => (
                toast.text.clone(),
                if toast.trouble { ink.block } else { ink.chosen },
            ),
            (None, None) => return,
        };
        let galley = ui.painter().layout(
            text,
            egui::TextStyle::Body.resolve(ui.style()),
            ink.text,
            (rect.width() - 40.0).max(120.0),
        );
        let size = galley.size() + egui::vec2(24.0, 14.0);
        let at = egui::pos2(rect.center().x - size.x / 2.0, rect.min.y + 12.0);
        let card = egui::Rect::from_min_size(at, size);
        ui.painter().rect(
            card,
            8.0,
            ink.card,
            egui::Stroke::new(1.5, color),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(at + egui::vec2(12.0, 7.0), galley, ink.text);
    }
}

impl eframe::App for FarmApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.clock = ctx.input(|input| input.time);
        let (close, outer, inner) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.close_requested(),
                viewport.outer_rect,
                viewport.inner_rect,
            )
        });
        if let (Some(outer), Some(inner)) = (outer, inner) {
            self.place = Some(WindowPlace {
                x: outer.min.x,
                y: outer.min.y,
                width: inner.width(),
                height: inner.height(),
            });
        }
        if close {
            self.leave();
        }
        self.listen(&ctx);
        self.keys(&ctx);
        if !ctx.input(|input| input.pointer.any_down()) {
            self.editor.settle();
        }
        self.keep_draft_when_paused();
        self.habitat(&ctx);

        let width = ctx.content_rect().width();
        let narrow = width < NARROW;
        let ink = style::ink(ui.visuals().dark_mode);
        let panel = egui::Frame::new()
            .fill(ink.paper)
            .inner_margin(egui::Margin::symmetric(12, 10));
        egui::Panel::top("farm-top")
            .frame(panel.stroke(egui::Stroke::new(1.0, ink.line)))
            .show(ui, |ui| self.top_bar(ui, narrow));
        egui::Panel::bottom("farm-strip")
            .frame(panel.stroke(egui::Stroke::new(1.0, ink.line)))
            .show(ui, |ui| self.strip(ui));
        if !narrow || self.rail_open {
            egui::Panel::left("farm-rail")
                .resizable(false)
                .exact_size(244.0)
                .frame(panel)
                .show(ui, |ui| self.rail(ui));
        }
        if !narrow || self.inspector_open {
            egui::Panel::right("farm-inspector")
                .resizable(false)
                .exact_size(320.0)
                .frame(panel)
                .show(ui, |ui| self.inspector(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                self.stage(ui, rect);
                self.toast(ui, rect);
            });
        self.dialogs(&ctx);
        self.take_snap(&ctx);
        let fps = if self.reduce_motion() { 4 } else { 30 };
        ctx.request_repaint_after(std::time::Duration::from_millis(1000 / fps));
    }

    /// A window opened only to have its picture taken answers to nobody.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.snap.is_some() {
            raw_input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.36, 0.25, 0.19, 1.0]
    }
}

impl Drop for FarmApp {
    fn drop(&mut self) {
        self.leave();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_forms::Form;

    fn app(edit: Option<usize>) -> FarmApp {
        let (rehearsal, snapshot) = crate::session::Rehearsal::open(
            formiga_farm_contract::sample::colony(),
            edit,
            "test".into(),
        )
        .unwrap();
        let ctx = egui::Context::default();
        FarmApp::new(
            &ctx,
            Opening {
                snapshot,
                host: Host::Rehearsal(Box::new(rehearsal)),
                store: Store::new(None),
                open: None,
                start: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn applying_a_new_look_is_kept_and_becomes_the_look_to_compare_against() {
        let mut app = app(Some(0));
        let orca = crate::presets::find("animal.orca").unwrap().design;
        app.editor
            .start_from(orca.clone(), Some("animal.orca".into()));
        app.apply();
        assert!(app.pending.is_some());
        app.listen(&egui::Context::default());
        assert!(app.pending.is_none());
        assert_eq!(app.editor.arrival, orca);
        assert_eq!(
            app.expected_revision.as_deref(),
            Some(orca.revision().as_str())
        );
        assert!(
            app.toast.as_ref().is_some_and(|t| !t.trouble),
            "told it was kept"
        );
        // Applying again from the new look is kept too: the revision moved on with it.
        app.editor.randomize(Layer::Finish, 9);
        app.apply();
        app.listen(&egui::Context::default());
        assert!(app.dialog.is_none());
        assert!(app.toast.as_ref().is_some_and(|t| !t.trouble));
    }

    #[test]
    fn a_new_formiga_is_handed_to_desktops_welcome() {
        let mut app = app(None);
        let panda = crate::presets::find("animal.giant_panda").unwrap().design;
        app.editor
            .start_from(panda, Some("animal.giant_panda".into()));
        app.apply();
        app.listen(&egui::Context::default());
        assert!(
            app.toast
                .as_ref()
                .is_some_and(|t| t.text.contains("welcoming"))
        );
        let Host::Rehearsal(rehearsal) = &app.host else {
            panic!()
        };
        assert_eq!(rehearsal.welcomed, 1);
    }

    #[test]
    fn nothing_is_applied_while_a_notice_blocks_it() {
        let mut app = app(None);
        app.snapshot
            .capabilities
            .retain(|c| *c != formiga_farm_contract::FarmCapability::SculptedForms);
        app.notices = (u64::MAX, Vec::new());
        let whale = crate::presets::find("animal.blue_whale").unwrap().design;
        assert!(matches!(whale.form, Form::Sculpted { .. }));
        app.editor.start_from(whale, None);
        app.apply();
        assert!(app.pending.is_none());
    }
}
