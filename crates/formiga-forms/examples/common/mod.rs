//! What the examples share: a frame blown up onto a sheet, and the sheet written as a PNG.
use formiga_art::Canvas;

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
