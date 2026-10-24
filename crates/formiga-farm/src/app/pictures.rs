//! Pictures of designs, drawn once and kept as textures until the design changes: every frame
//! the stage plays, and a small picture of every preset and draft on the shelves. A sculpted
//! form is shown in Farm's own high definition wherever it is shown large, and in Desktop's
//! 48 pixels wherever Farm shows how it will look in Desktop, Home or Hill.

use eframe::egui;
use formiga_art::Canvas;
use formiga_art::forms::{DesignRenderer, Intent};
use formiga_core::AppearanceGenome;
use formiga_core::forms::{Design, Form};
use std::borrow::Cow;
use std::collections::HashMap;

pub fn texture(ctx: &egui::Context, name: &str, canvas: &Canvas) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        &canvas.rgba_bytes(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::NEAREST)
}

/// Whether a design is drawn by Farm in high definition: a sculpted form is, and a creature
/// that keeps its body is always Desktop's own pixels.
pub fn drawn_hd(design: &Design) -> bool {
    matches!(design.form, Form::Sculpted { .. })
}

/// The design key of the frames of a design shown beside the workbench's, to compare.
pub const COMPARED: u64 = u64::MAX - 1;

/// Which frame of which design.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameKey {
    /// The editor's change count, or `u64::MAX` for the design as it was when the session
    /// opened.
    pub design: u64,
    pub intent: Intent,
    pub frame: u8,
    pub facing_right: bool,
    pub outlined: bool,
    /// Pixels to each of the frame's: 1 for Desktop's own frame, more for high definition.
    pub detail: u32,
    /// Drawn as Formiga Hill and Home will draw it ([`crate::drawing::as_it_travels`]).
    pub travels: bool,
    /// Wearing what the creature wears in Desktop.
    pub dressed: bool,
}

/// Which design's resting baseline: the editor's change count or one of the other design keys
/// of [`FrameKey::design`], whether it is drawn as it travels, and whether motion is reduced.
type BaselineKey = (u64, bool, bool);

#[derive(Default)]
pub struct Pictures {
    frames: HashMap<FrameKey, egui::TextureHandle>,
    /// How far above the bottom of its frame each design stands at rest, worked out once: for a
    /// sculpted form that is a whole drawing, and for a companion every frame of several poses.
    baselines: HashMap<BaselineKey, u32>,
    /// The change count the frames were drawn for.
    drawn_for: u64,
    thumbs: HashMap<String, (egui::TextureHandle, egui::Vec2)>,
    /// Small pictures of the design on the workbench with one thing changed, for choosing
    /// among: let go whenever the design changes.
    variants: HashMap<String, (egui::TextureHandle, egui::Vec2)>,
    variants_for: u64,
}

impl Pictures {
    /// One frame of a design, drawn if it has not been already, and how far above the bottom of
    /// the frame the design stands at rest. `design` is asked for only when something has to be
    /// drawn or worked out, so a design shown again and again is not made again each time.
    #[allow(clippy::too_many_arguments)]
    pub fn frame<'d>(
        &mut self,
        ctx: &egui::Context,
        base: &AppearanceGenome,
        key: FrameKey,
        current: u64,
        reduce_motion: bool,
        dress: Option<formiga_art::AccessoryArt>,
        design: impl FnOnce() -> Cow<'d, Design>,
    ) -> (egui::TextureHandle, u32) {
        if self.drawn_for != current {
            let kept = |design: u64| design == u64::MAX || design == COMPARED;
            self.frames.retain(|key, _| kept(key.design));
            self.baselines.retain(|(design, _, _), _| kept(*design));
            self.drawn_for = current;
        }
        let baseline_key = (key.design, key.travels, reduce_motion);
        if let (Some(texture), Some(baseline)) =
            (self.frames.get(&key), self.baselines.get(&baseline_key))
        {
            return (texture.clone(), *baseline);
        }
        let design = design();
        let baseline = *self
            .baselines
            .entry(baseline_key)
            .or_insert_with(|| DesignRenderer::resting_baseline(&design, base, reduce_motion));
        if let Some(texture) = self.frames.get(&key) {
            return (texture.clone(), baseline);
        }
        // A creature that keeps its body is Desktop's own pixels, drawn larger on screen.
        let detail = if key.detail > 1 && drawn_hd(&design) {
            key.detail
        } else {
            1
        };
        let mut canvas = crate::drawing::dressed_intent_frame(
            &design,
            base,
            dress.filter(|_| key.dressed),
            key.intent,
            key.frame,
            key.facing_right,
            reduce_motion,
            detail,
        );
        if key.outlined && detail == 1 {
            formiga_art::CreatureRenderer::outline_frame(&mut canvas);
        }
        let texture = texture(ctx, "farm-frame", &canvas);
        if self.frames.len() > 256 {
            self.frames.clear();
        }
        self.frames.insert(key, texture.clone());
        (texture, baseline)
    }

    /// Let go of the frames of a design shown beside the workbench's, which are kept as
    /// [`COMPARED`].
    pub fn forget_compared(&mut self) {
        self.frames.retain(|key, _| key.design != COMPARED);
        self.baselines
            .retain(|(design, _, _), _| *design != COMPARED);
    }

    /// Forget the frames of the design as it was, when that changes.
    pub fn forget_before(&mut self) {
        self.frames.retain(|key, _| key.design != u64::MAX);
        self.baselines
            .retain(|(design, _, _), _| *design != u64::MAX);
    }

    /// Let go of the small pictures whose names `keep` no longer wants.
    pub fn keep_thumbs(&mut self, keep: impl Fn(&str) -> bool) {
        self.thumbs.retain(|name, _| keep(name));
    }

    /// Whether a small picture is kept under `name`.
    #[cfg(test)]
    pub fn has_thumb(&self, name: &str) -> bool {
        self.thumbs.contains_key(name)
    }

    /// A small picture of `design` at rest, cropped to the creature, kept under `name`.
    pub fn thumb(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        design: &Design,
        base: &AppearanceGenome,
    ) -> (egui::TextureHandle, egui::Vec2) {
        if let Some(thumb) = self.thumbs.get(name) {
            return thumb.clone();
        }
        let hd = drawn_hd(design);
        let (canvas, detail) = if hd {
            (
                crate::drawing::intent_frame_hd(
                    design,
                    base,
                    Intent::Idle,
                    0,
                    true,
                    true,
                    crate::drawing::DETAIL,
                ),
                crate::drawing::DETAIL as f32,
            )
        } else {
            (
                DesignRenderer::intent_frame(design, base, Intent::Idle, 0, true, true),
                1.0,
            )
        };
        let cropped = crop(&canvas);
        // Measured in the frame's own pixels, however finely it is drawn.
        let size = egui::vec2(
            cropped.width() as f32 / detail,
            cropped.height() as f32 / detail,
        );
        let texture = texture(ctx, name, &cropped);
        let thumb = (texture, size);
        self.thumbs.insert(name.to_owned(), thumb.clone());
        thumb
    }
}

impl Pictures {
    /// A small picture of `design`, one of several variants of the design on the workbench to
    /// choose among, named `name` and drawn at rest in Desktop's own pixels: the whole creature,
    /// or only its face when `face` is set. Variants are let go when the workbench's change count
    /// `current` moves on.
    pub fn variant(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        design: &Design,
        base: &AppearanceGenome,
        current: u64,
        face: bool,
    ) -> (egui::TextureHandle, egui::Vec2) {
        if self.variants_for != current {
            self.variants.clear();
            self.variants_for = current;
        }
        if let Some(variant) = self.variants.get(name) {
            return variant.clone();
        }
        let frame = DesignRenderer::intent_frame(design, base, Intent::Idle, 0, true, true);
        let canvas = if face {
            // The face's own 16 pixels, and a little of the head round them.
            let at = DesignRenderer::anchors(design, base).face;
            let mut out = Canvas::new(FACE_AREA, FACE_AREA);
            let half = FACE_AREA as i32 / 2;
            for y in 0..FACE_AREA as i32 {
                for x in 0..FACE_AREA as i32 {
                    out.set(x, y, frame.get(at.x - half + x, at.y - half + y));
                }
            }
            out
        } else {
            crop(&frame)
        };
        let size = egui::vec2(canvas.width() as f32, canvas.height() as f32);
        let variant = (texture(ctx, name, &canvas), size);
        self.variants.insert(name.to_owned(), variant.clone());
        variant
    }
}

/// How many frame pixels across a picture of a face shows.
const FACE_AREA: u32 = 20;

/// `canvas` cropped to what is drawn on it.
fn crop(canvas: &Canvas) -> Canvas {
    let Some((left, top, right, bottom)) = canvas.alpha_bounds() else {
        return canvas.clone();
    };
    let (w, h) = (right - left + 1, bottom - top + 1);
    let mut out = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.set(
                x as i32,
                y as i32,
                canvas.get((left + x) as i32, (top + y) as i32),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(design: u64, frame: u8) -> FrameKey {
        FrameKey {
            design,
            intent: Intent::Idle,
            frame,
            facing_right: true,
            outlined: false,
            detail: 1,
            travels: false,
            dressed: false,
        }
    }

    #[test]
    fn a_design_is_drawn_and_its_baseline_worked_out_once_until_it_changes() {
        let ctx = egui::Context::default();
        let base = crate::review::stand_in();
        let whale = crate::presets::find("animal.blue_whale").unwrap().design;
        let mut pictures = Pictures::default();
        let asked = std::cell::Cell::new(0);
        let show = |pictures: &mut Pictures, key: FrameKey, current: u64| {
            pictures
                .frame(&ctx, &base, key, current, false, None, || {
                    asked.set(asked.get() + 1);
                    Cow::Borrowed(&whale)
                })
                .1
        };
        let baseline = show(&mut pictures, key(1, 0), 1);
        assert_eq!(
            baseline,
            DesignRenderer::resting_baseline(&whale, &base, false)
        );
        // The same frame again, on every frame the window draws, asks for nothing.
        assert_eq!(show(&mut pictures, key(1, 0), 1), baseline);
        assert_eq!(show(&mut pictures, key(1, 0), 1), baseline);
        assert_eq!(asked.get(), 1);
        // Another frame of the pose is drawn, but the baseline is the one already worked out.
        assert_eq!(show(&mut pictures, key(1, 1), 1), baseline);
        assert_eq!(asked.get(), 2);
        assert_eq!(pictures.baselines.len(), 1);
        // Once the design changes, it is drawn and worked out afresh.
        show(&mut pictures, key(2, 0), 2);
        assert_eq!(asked.get(), 3);
        assert_eq!(pictures.baselines.len(), 1);
        assert!(pictures.frames.keys().all(|key| key.design == 2));
    }

    #[test]
    fn small_pictures_no_longer_wanted_are_let_go() {
        let ctx = egui::Context::default();
        let base = crate::review::stand_in();
        let blank = crate::presets::blank();
        let mut pictures = Pictures::default();
        let _ = pictures.thumb(&ctx, "kept", &blank, &base);
        let _ = pictures.thumb(&ctx, "stale", &blank, &base);
        pictures.keep_thumbs(|name| name == "kept");
        assert_eq!(pictures.thumbs.keys().collect::<Vec<_>>(), ["kept"]);
    }
}
