//! Pictures drawn without a window, for review: every preset, and one design in every pose.

use anyhow::{Context, Result};
use formiga_art::{Canvas, Rgba};
use formiga_core::AppearanceGenome;
use formiga_forms::{Design, DesignRenderer, Intent};
use std::path::Path;

/// The genome a design is drawn over when it is not yet anyone's: an ordinary grown companion.
pub fn stand_in() -> AppearanceGenome {
    formiga_farm_contract::sample::stand_in()
}

/// `canvas`, each pixel `scale` across.
pub fn scaled(canvas: &Canvas, scale: i32) -> Canvas {
    let mut out = Canvas::new(
        canvas.width() * scale as u32,
        canvas.height() * scale as u32,
    );
    blit(&mut out, canvas, 0, 0, scale);
    out
}

/// Copy `source` into `target` at `(x, y)`, each pixel `scale` across, laid over what is
/// there where it is see-through.
pub fn blit(target: &mut Canvas, source: &Canvas, x: i32, y: i32, scale: i32) {
    for sy in 0..source.height() as i32 {
        for sx in 0..source.width() as i32 {
            let pixel = source.get(sx, sy);
            if pixel.a == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let (tx, ty) = (x + sx * scale + dx, y + sy * scale + dy);
                    let color = if pixel.a == 255 {
                        pixel
                    } else {
                        let under = target.get(tx, ty);
                        let a = u16::from(pixel.a);
                        let mix = |c: u8, u: u8| {
                            ((u16::from(c) * a + u16::from(u) * (255 - a)) / 255) as u8
                        };
                        Rgba::new(
                            mix(pixel.r, under.r),
                            mix(pixel.g, under.g),
                            mix(pixel.b, under.b),
                            under.a.max(pixel.a),
                        )
                    };
                    target.set(tx, ty, color);
                }
            }
        }
    }
}

/// One frame of `intent`, and how many times larger to copy it to show each frame pixel
/// `scale` across: `detail` pixels to each frame pixel, 1 being the 48 pixels of a companion.
fn picture(design: &Design, intent: Intent, frame: u8, scale: i32, detail: u32) -> (Canvas, i32) {
    let base = stand_in();
    if detail > 1 {
        let canvas =
            DesignRenderer::intent_frame_hd(design, &base, intent, frame, true, false, detail);
        let drawn = canvas.width() as i32 / 48;
        (canvas, (scale / drawn).max(1))
    } else {
        (
            DesignRenderer::intent_frame(design, &base, intent, frame, true, false),
            scale,
        )
    }
}

/// The paper behind cell `n` of a sheet, light and dark by turns like a chequerboard.
fn shade(n: i32) -> Rgba {
    if n % 2 == 0 {
        Rgba::new(238, 232, 216, 255)
    } else {
        Rgba::new(224, 216, 198, 255)
    }
}

/// Every preset at rest and walking, a row of each: what `--render-presets` draws.
pub fn presets_sheet(scale: i32, detail: u32) -> Canvas {
    let presets = crate::presets::all();
    let columns = 8;
    let cell = 48 * scale;
    let rows = presets.len().div_ceil(columns) as i32;
    let mut canvas = Canvas::new((columns as i32 * cell * 2) as u32, (rows * cell) as u32);
    for (index, preset) in presets.iter().enumerate() {
        let (col, row) = ((index % columns) as i32, (index / columns) as i32);
        for (pane, intent) in [Intent::Idle, Intent::Move].into_iter().enumerate() {
            let x = (col * 2 + pane as i32) * cell;
            let y = row * cell;
            canvas.fill_rect(x, y, cell, cell, shade(col + row));
            let (frame, by) = picture(
                &preset.design,
                intent,
                if pane == 0 { 0 } else { 2 },
                scale,
                detail,
            );
            blit(&mut canvas, &frame, x, y, by);
        }
    }
    canvas
}

/// One design in every intent, every frame of each: what `--render-poses` draws.
pub fn poses_sheet(design: &Design, scale: i32, detail: u32) -> Canvas {
    let cell = 48 * scale;
    let columns = 6;
    let mut canvas = Canvas::new(
        (columns * cell) as u32,
        (Intent::ALL.len() as i32 * cell) as u32,
    );
    for (row, intent) in Intent::ALL.into_iter().enumerate() {
        let (frames, _) = DesignRenderer::timing(intent);
        for frame in 0..frames.min(columns as u8) {
            let (x, y) = (i32::from(frame) * cell, row as i32 * cell);
            canvas.fill_rect(x, y, cell, cell, shade(row as i32 + i32::from(frame)));
            let (picture, by) = picture(design, intent, frame, scale, detail);
            blit(&mut canvas, &picture, x, y, by);
        }
    }
    canvas
}

/// A picture and the takes read from it, side by side: what `--import` draws.
pub fn import_sheet(picture: &image::RgbaImage, takes: &[crate::import::Take]) -> Canvas {
    let scale = 4;
    let cell = 48 * scale;
    let mut canvas = Canvas::new(((takes.len() + 1) as i32 * cell) as u32, cell as u32);
    for pane in 0..=takes.len() as i32 {
        canvas.fill_rect(pane * cell, 0, cell, cell, shade(pane));
    }
    // The picture, fitted into the first pane.
    let fit = cell as f32 / picture.width().max(picture.height()) as f32;
    let (w, h) = (
        (picture.width() as f32 * fit) as i32,
        (picture.height() as f32 * fit) as i32,
    );
    for y in 0..h {
        for x in 0..w {
            let sx = ((x as f32 + 0.5) / fit) as u32;
            let sy = ((y as f32 + 0.5) / fit) as u32;
            let [r, g, b, a] = picture
                .get_pixel(sx.min(picture.width() - 1), sy.min(picture.height() - 1))
                .0;
            if a > 0 {
                let mut one = Canvas::new(1, 1);
                one.set(0, 0, Rgba::new(r, g, b, a));
                blit(&mut canvas, &one, (cell - w) / 2 + x, cell - h + y, 1);
            }
        }
    }
    for (index, take) in takes.iter().enumerate() {
        let (frame, by) = picture_of(&take.design, scale);
        blit(&mut canvas, &frame, (index as i32 + 1) * cell, 0, by);
    }
    canvas
}

fn picture_of(design: &Design, scale: i32) -> (Canvas, i32) {
    picture(design, Intent::Idle, 0, scale, formiga_forms::DETAIL)
}

pub fn write_png(canvas: &Canvas, path: &Path) -> Result<()> {
    std::fs::write(path, crate::icon::png(canvas))
        .with_context(|| format!("could not write {}", path.display()))
}
