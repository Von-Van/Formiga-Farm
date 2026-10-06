//! A review sheet of every starter in every intent: `cargo run -p formiga-forms --example sheet -- out.png`
use formiga_art::Canvas;
use formiga_forms::{Design, DesignRenderer, Form, Intent, Plan, Sculpt, plain_face};

#[allow(dead_code)]
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sheet.png".into());
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    let scale = 4;
    let frames = 6;
    let cell = 48 * scale;
    let rows = Plan::ALL.len();
    let cols = Intent::ALL.len() * 2;
    let mut canvas = Canvas::new((cols * cell) as u32, (rows * cell) as u32);
    for (row, plan) in Plan::ALL.into_iter().enumerate() {
        let design = Design {
            form: Form::Sculpted {
                sculpt: Sculpt::starter(plan),
            },
            face: plain_face(),
        };
        for (i, intent) in Intent::ALL.into_iter().enumerate() {
            let (count, _) = DesignRenderer::timing(intent);
            for (j, frame) in [0u8, (count / 2).min(frames)].into_iter().enumerate() {
                let f = DesignRenderer::intent_frame(&design, &base, intent, frame, true, false);
                blit(
                    &mut canvas,
                    &f,
                    (i * 2 + j) * cell,
                    row * cell,
                    scale,
                    (i + row) % 2 == 0,
                );
            }
        }
    }
    write(&canvas, &out);
}

pub fn blit(target: &mut Canvas, source: &Canvas, ox: usize, oy: usize, scale: usize, tint: bool) {
    let bg = if tint {
        formiga_art::Rgba::new(236, 230, 214, 255)
    } else {
        formiga_art::Rgba::new(222, 214, 196, 255)
    };
    for y in 0..48 * scale {
        for x in 0..48 * scale {
            target.set((ox + x) as i32, (oy + y) as i32, bg);
        }
    }
    for y in 0..source.height() as usize {
        for x in 0..source.width() as usize {
            let p = source.get(x as i32, y as i32);
            if p.a > 0 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        target.set(
                            (ox + x * scale + dx) as i32,
                            (oy + y * scale + dy) as i32,
                            p,
                        );
                    }
                }
            }
        }
    }
}

pub fn write(canvas: &Canvas, path: &str) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        canvas.width(),
        canvas.height(),
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&canvas.rgba_bytes())
        .unwrap();
}
