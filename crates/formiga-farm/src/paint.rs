//! Small colour helpers for Farm's own pictures.

use formiga_art::Rgba;

pub fn rgb(hex: u32) -> Rgba {
    Rgba::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

/// `a`, `t` of the way (out of 255) toward `b`.
pub fn blend(a: Rgba, b: Rgba, t: u8) -> Rgba {
    let mix = |x: u8, y: u8| {
        ((u16::from(x) * u16::from(255 - t) + u16::from(y) * u16::from(t)) / 255) as u8
    };
    Rgba::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), a.a)
}

pub fn darker(c: Rgba, by: u8) -> Rgba {
    Rgba::new(
        c.r.saturating_sub(by),
        c.g.saturating_sub(by),
        c.b.saturating_sub(by),
        c.a,
    )
}

pub fn lighter(c: Rgba, by: u8) -> Rgba {
    Rgba::new(
        c.r.saturating_add(by),
        c.g.saturating_add(by),
        c.b.saturating_add(by),
        c.a,
    )
}

/// One channel of a colour as light, from 0 to 1, rather than as it is stored.
pub fn to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// How light a colour looks, from 0 to 1.
pub fn luminance(c: [u8; 3]) -> f32 {
    0.2126 * to_linear(c[0]) + 0.7152 * to_linear(c[1]) + 0.0722 * to_linear(c[2])
}

/// How far apart two colours are to the eye: 1 for the same, 21 for black on white.
pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}
