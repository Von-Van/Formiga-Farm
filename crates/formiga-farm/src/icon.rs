//! Farm's icon: a slice of the ant farm in its wooden frame, with a Formiga sitting on the grass
//! above the tunnels. It is drawn once as pixel art, 32 pixels square, and every size is made from
//! that drawing by whole pixels, so each stays crisp. The same picture is the window's icon, the
//! macOS bundle's `.icns` and the Windows installer's `.ico`; the packaging scripts ask the binary
//! for them, so there is one source.

// The frame, the soil and the tunnels are in the habitat's own colours, so the icon is the
// window's ant farm seen from further off.
use crate::habitat::{STRATA, TUNNEL, WOOD};
use crate::paint::{blend, darker, lighter, rgb};
use formiga_art::Canvas;

/// How big the drawing is, in pixels each way.
const SIZE: i32 = 32;

/// The row the grass grows along.
const GROUND: i32 = 13;

/// The picture, at its own size.
fn picture() -> Canvas {
    let mut canvas = Canvas::new(SIZE as u32, SIZE as u32);
    // How far a pixel is inside a square with its corners rounded on a radius of five, or `None`
    // outside it.
    let depth = |x: i32, y: i32| -> Option<i32> {
        let corner = |a: i32| (5 - a).max(a - (SIZE - 6)).max(0);
        let (cx, cy) = (corner(x), corner(y));
        let round = ((cx * cx + cy * cy) as f32).sqrt();
        if round > 5.5 {
            return None;
        }
        let straight = x.min(y).min(SIZE - 1 - x).min(SIZE - 1 - y);
        Some(if cx > 0 && cy > 0 {
            (5.5 - round) as i32
        } else {
            straight
        })
    };

    for y in 0..SIZE {
        for x in 0..SIZE {
            let Some(depth) = depth(x, y) else { continue };
            let color = if depth < 3 {
                // The frame, lit from the upper left: its top and left sides catch the light.
                let lit = x + y < SIZE;
                match depth {
                    0 => rgb(WOOD[2]),
                    1 if lit => lighter(rgb(WOOD[1]), 12),
                    1 => rgb(WOOD[0]),
                    _ => darker(rgb(WOOD[2]), 10),
                }
            } else if y < GROUND {
                blend(
                    rgb(0xa9cfe3),
                    rgb(0xe3eef0),
                    ((y - 3) * 255 / (GROUND - 3)) as u8,
                )
            } else {
                // Soil in wavy bands, deeper and lighter as it goes down.
                let wave = (x / 3) % 3 - 1;
                let band = if y + wave < 19 {
                    0
                } else if y + wave < 24 {
                    1
                } else {
                    2
                };
                let mut color = rgb(STRATA[band]);
                if (x * 7 + y * 13) % 17 == 0 {
                    color = darker(color, 16);
                }
                color
            };
            canvas.set(x, y, color);
        }
    }

    // A tunnel from the grass down to a chamber, and on to another below it.
    let tunnel = rgb(TUNNEL);
    for (x, y) in [
        (8, 14),
        (8, 15),
        (9, 16),
        (9, 17),
        (10, 18),
        (11, 19),
        (12, 20),
        (13, 21),
        (14, 21),
        (22, 23),
        (23, 24),
        (24, 25),
        (25, 26),
    ] {
        canvas.set(x, y, tunnel);
        canvas.set(x + 1, y, tunnel);
    }
    for dy in -2..=2_i32 {
        for dx in -4..=4_i32 {
            if dx * dx * 4 + dy * dy * 16 <= 64 + 4 {
                let floor = dy == 2;
                canvas.set(18 + dx, 22 + dy, if floor { rgb(0x684a39) } else { tunnel });
            }
        }
    }
    // Seeds stored on the chamber floor, and an ant on its way down.
    for x in [16, 18, 20] {
        canvas.set(x, 23, rgb(0xe6d3a3));
        canvas.set(x + 1, 23, darker(rgb(0xe6d3a3), 30));
    }
    let ant = rgb(0x2a2433);
    canvas.set(9, 17, ant);
    canvas.set(10, 18, ant);

    // Grass along the top of the soil.
    for x in 3..SIZE - 3 {
        let tuft = if x % 4 == 1 { 1 } else { 0 };
        for y in GROUND - tuft..=GROUND {
            canvas.set(
                x,
                y,
                if (x + y) % 3 == 0 {
                    rgb(0x6f8f4e)
                } else {
                    rgb(0x7fa05a)
                },
            );
        }
        canvas.set(x, GROUND + 1, rgb(0x5f7a42));
    }
    canvas.set(8, GROUND, tunnel);
    canvas.set(9, GROUND, tunnel);

    formiga(&mut canvas);

    canvas
}

/// A Formiga sitting on the grass and looking out, drawn pixel by pixel: edged in a darker shade
/// of its own coat, lit from the upper left, with its ears up and a blush.
const FORMIGA: [&str; 10] = [
    ".oo......oo.",
    "ohho....ocso",
    "ohccoooocsso",
    "ohcccccccsso",
    "ohcwkccwkcso",
    "occkkcckkcso",
    "ocbccmmccbso",
    "occcccccccso",
    ".osssssssso.",
    "..oo....oo..",
];

fn formiga(canvas: &mut Canvas) {
    let coat = rgb(0xe9a46f);
    let left = 15;
    let top = GROUND - FORMIGA.len() as i32;
    for (y, row) in FORMIGA.iter().enumerate() {
        for (x, key) in row.chars().enumerate() {
            let color = match key {
                'o' => blend(coat, rgb(0x3b2430), 190),
                'h' => lighter(coat, 14),
                'c' => coat,
                's' => darker(coat, 30),
                'w' => rgb(0xfdf6ec),
                'k' => rgb(0x2a2433),
                'b' => blend(coat, rgb(0xd9607a), 110),
                'm' => darker(coat, 80),
                _ => continue,
            };
            canvas.set(left + x as i32, top + y as i32, color);
        }
    }
}

/// The icon at `size` pixels square: the picture scaled up by whole pixels and centred, or, for
/// the few sizes smaller than it, every other pixel of it.
pub fn at(size: u32) -> Canvas {
    let small = picture();
    let mut canvas = Canvas::new(size, size);
    if size < SIZE as u32 {
        let step = SIZE as f32 / size as f32;
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                let (sx, sy) = ((x as f32 * step) as i32, (y as f32 * step) as i32);
                canvas.set(x, y, small.get(sx, sy));
            }
        }
        return canvas;
    }
    let scale = size as i32 / SIZE;
    let offset = (size as i32 - SIZE * scale) / 2;
    for y in 0..SIZE * scale {
        for x in 0..SIZE * scale {
            canvas.set(x + offset, y + offset, small.get(x / scale, y / scale));
        }
    }
    canvas
}

/// The icon as the window's own, for the title bar and the task bar.
pub fn window() -> eframe::egui::IconData {
    let picture = at(64);
    eframe::egui::IconData {
        rgba: picture.rgba_bytes(),
        width: picture.width(),
        height: picture.height(),
    }
}

/// A canvas as PNG bytes.
pub fn png(canvas: &Canvas) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, canvas.width(), canvas.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("a PNG header writes to memory");
        writer
            .write_image_data(&canvas.rgba_bytes())
            .expect("a PNG writes to memory");
    }
    bytes
}

/// A macOS `.icns`: PNGs at 128, 256, 512 and 1024 pixels.
pub fn icns() -> Vec<u8> {
    let entries: Vec<(&[u8; 4], Vec<u8>)> = [
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
    ]
    .into_iter()
    .map(|(kind, size)| (kind, png(&at(size))))
    .collect();
    let length = 8 + entries
        .iter()
        .map(|(_, data)| 8 + data.len())
        .sum::<usize>();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(b"icns");
    bytes.extend_from_slice(&(length as u32).to_be_bytes());
    for (kind, data) in entries {
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
        bytes.extend_from_slice(&data);
    }
    bytes
}

/// A Windows `.ico`: PNGs at 16, 32, 48 and 256 pixels.
pub fn ico() -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = [16, 32, 48, 256]
        .into_iter()
        .map(|size| (size, png(&at(size))))
        .collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0, 0, 1, 0]);
    bytes.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, data) in &images {
        // A side of 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        bytes.extend_from_slice(&[side, side, 0, 0]);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in images {
        bytes.extend_from_slice(&data);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_is_the_same_picture_at_every_size() {
        let small = at(32);
        let big = at(256);
        for (x, y) in [(20, 8), (18, 22), (9, 17), (4, 4), (15, 27)] {
            assert_eq!(small.get(x, y), big.get(x * 8 + 3, y * 8 + 3), "({x}, {y})");
        }
        let tiny = at(16);
        for (x, y) in [(10, 4), (9, 11)] {
            assert_eq!(tiny.get(x, y), small.get(x * 2, y * 2));
        }
        assert_eq!(small.get(0, 0).a, 0, "the corners stay clear");
        assert_eq!(small.get(16, 0).a, 255, "the frame reaches the edge");
    }

    #[test]
    fn the_icon_files_say_what_they_hold() {
        let icns = icns();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        let ico = ico();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 4);
    }

    #[test]
    fn the_window_icon_is_the_picture_at_sixty_four_pixels() {
        let icon = window();
        assert_eq!((icon.width, icon.height), (64, 64));
        assert_eq!(icon.rgba, at(64).rgba_bytes());
    }
}
