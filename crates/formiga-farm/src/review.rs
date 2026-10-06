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

/// Copy `source` into `target` at `(x, y)`, each pixel `scale` across.
pub fn blit(target: &mut Canvas, source: &Canvas, x: i32, y: i32, scale: i32) {
    for sy in 0..source.height() as i32 {
        for sx in 0..source.width() as i32 {
            let pixel = source.get(sx, sy);
            if pixel.a == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    target.set(x + sx * scale + dx, y + sy * scale + dy, pixel);
                }
            }
        }
    }
}

fn fill(target: &mut Canvas, x: i32, y: i32, w: i32, h: i32, color: Rgba) {
    target.fill_rect(x, y, w, h, color);
}

/// Every preset at rest and walking, a row of each: what `--render-presets` draws.
pub fn presets_sheet(scale: i32) -> Canvas {
    let presets = crate::presets::all();
    let columns = 8;
    let cell = 48 * scale;
    let rows = presets.len().div_ceil(columns) as i32;
    let mut canvas = Canvas::new((columns as i32 * cell * 2) as u32, (rows * cell) as u32);
    let base = stand_in();
    for (index, preset) in presets.iter().enumerate() {
        let (col, row) = ((index % columns) as i32, (index / columns) as i32);
        for (pane, intent) in [Intent::Idle, Intent::Move].into_iter().enumerate() {
            let x = (col * 2 + pane as i32) * cell;
            let y = row * cell;
            let shade = if (col + row) % 2 == 0 {
                Rgba::new(238, 232, 216, 255)
            } else {
                Rgba::new(224, 216, 198, 255)
            };
            fill(&mut canvas, x, y, cell, cell, shade);
            let frame = DesignRenderer::intent_frame(
                &preset.design,
                &base,
                intent,
                if pane == 0 { 0 } else { 2 },
                true,
                false,
            );
            blit(&mut canvas, &frame, x, y, scale);
        }
    }
    canvas
}

/// One design in every intent, every frame of each: what `--render-poses` draws.
pub fn poses_sheet(design: &Design, scale: i32) -> Canvas {
    let cell = 48 * scale;
    let base = stand_in();
    let columns = 6;
    let mut canvas = Canvas::new(
        (columns * cell) as u32,
        (Intent::ALL.len() as i32 * cell) as u32,
    );
    for (row, intent) in Intent::ALL.into_iter().enumerate() {
        let (frames, _) = DesignRenderer::timing(intent);
        for frame in 0..frames.min(columns as u8) {
            let (x, y) = (i32::from(frame) * cell, row as i32 * cell);
            let shade = if (row as i32 + i32::from(frame)) % 2 == 0 {
                Rgba::new(238, 232, 216, 255)
            } else {
                Rgba::new(224, 216, 198, 255)
            };
            fill(&mut canvas, x, y, cell, cell, shade);
            let picture = DesignRenderer::intent_frame(design, &base, intent, frame, true, false);
            blit(&mut canvas, &picture, x, y, scale);
        }
    }
    canvas
}

pub fn write_png(canvas: &Canvas, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("could not write {}", path.display()))?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        canvas.width(),
        canvas.height(),
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()?
        .write_image_data(&canvas.rgba_bytes())?;
    Ok(())
}
