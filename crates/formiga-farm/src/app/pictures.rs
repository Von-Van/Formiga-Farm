//! Pictures of designs, drawn once and kept as textures until the design changes: every frame
//! the stage plays, and a small picture of every preset and draft on the shelves.

use eframe::egui;
use formiga_art::Canvas;
use formiga_core::AppearanceGenome;
use formiga_forms::{Design, DesignRenderer, Intent};
use std::collections::HashMap;

pub fn texture(ctx: &egui::Context, name: &str, canvas: &Canvas) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        &canvas.rgba_bytes(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::NEAREST)
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
    #[allow(clippy::too_many_arguments)]
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
        let texture = texture(ctx, "farm-frame", &canvas);
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
        let canvas = DesignRenderer::intent_frame(design, base, Intent::Idle, 0, true, true);
        let cropped = match canvas.alpha_bounds() {
            Some((left, top, right, bottom)) => {
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
            None => canvas,
        };
        let size = egui::vec2(cropped.width() as f32, cropped.height() as f32);
        let thumb = (texture(ctx, name, &cropped), size);
        self.thumbs.insert(name.to_owned(), thumb.clone());
        thumb
    }
}
