//! Pictures of designs, drawn once and kept as textures until the design changes: every frame
//! the stage plays, and a small picture of every preset and draft on the shelves. A sculpted
//! form is shown in Farm's own high definition wherever it is shown large, and in Desktop's
//! 48 pixels wherever Farm shows how it will look in Desktop, Home or Hill.

use eframe::egui;
use formiga_art::Canvas;
use formiga_core::AppearanceGenome;
use formiga_forms::{Design, DesignRenderer, Form, Intent};
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
}

#[derive(Default)]
pub struct Pictures {
    frames: HashMap<FrameKey, egui::TextureHandle>,
    /// The change count the frames were drawn for.
    drawn_for: u64,
    thumbs: HashMap<String, (egui::TextureHandle, egui::Vec2)>,
}

impl Pictures {
    /// One frame of `design`, drawn if it has not been already.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        design: &Design,
        base: &AppearanceGenome,
        key: FrameKey,
        current: u64,
        reduce_motion: bool,
    ) -> egui::TextureHandle {
        if self.drawn_for != current {
            self.frames.retain(|key, _| key.design == u64::MAX);
            self.drawn_for = current;
        }
        if let Some(texture) = self.frames.get(&key) {
            return texture.clone();
        }
        let texture = if key.detail > 1 && drawn_hd(design) {
            let canvas = DesignRenderer::intent_frame_hd(
                design,
                base,
                key.intent,
                key.frame,
                key.facing_right,
                reduce_motion,
                key.detail,
            );
            texture(ctx, "farm-frame", &canvas)
        } else {
            let mut canvas = DesignRenderer::intent_frame(
                design,
                base,
                key.intent,
                key.frame,
                key.facing_right,
                reduce_motion,
            );
            if key.outlined {
                formiga_art::CreatureRenderer::outline_frame(&mut canvas);
            }
            texture(ctx, "farm-frame", &canvas)
        };
        if self.frames.len() > 256 {
            self.frames.clear();
        }
        self.frames.insert(key, texture.clone());
        texture
    }

    /// Forget the frames of the design as it was, when that changes.
    pub fn forget_before(&mut self) {
        self.frames.retain(|key, _| key.design != u64::MAX);
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
                DesignRenderer::intent_frame_hd(
                    design,
                    base,
                    Intent::Idle,
                    0,
                    true,
                    true,
                    formiga_forms::DETAIL,
                ),
                formiga_forms::DETAIL as f32,
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
