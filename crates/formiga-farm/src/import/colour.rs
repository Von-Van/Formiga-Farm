//! Colour arithmetic: sRGB, linear light and Lab, and the handful of colours a picture's
//! colours gather round.

use crate::paint::to_linear;

/// A colour set so that Farm's softening (three parts colour to one of light) brings it back
/// to `rgb`, as near as softening allows.
pub(super) fn unsoften(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| {
        ((i32::from(c) * 4 - 220) as f32 / 3.0)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

fn from_linear(c: f32) -> u8 {
    let c = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

const WHITE: [f32; 3] = [0.950_47, 1.0, 1.088_83];

pub(super) fn to_lab(rgb: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = rgb.map(to_linear);
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = 0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b;
    let f = |t: f32| {
        if t > 0.008_856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x / WHITE[0]), f(y / WHITE[1]), f(z / WHITE[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

pub(super) fn from_lab(lab: [f32; 3]) -> [u8; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    let f = |t: f32| {
        if t.powi(3) > 0.008_856 {
            t.powi(3)
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let (x, y, z) = (f(fx) * WHITE[0], f(fy) * WHITE[1], f(fz) * WHITE[2]);
    let r = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let g = -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z;
    let b = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    [from_linear(r), from_linear(g), from_linear(b)]
}

/// How different two colours look.
pub(super) fn delta(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub(super) fn chroma(lab: [f32; 3]) -> f32 {
    (lab[1] * lab[1] + lab[2] * lab[2]).sqrt()
}

pub(super) fn nearest(centres: &[[f32; 3]], c: [f32; 3]) -> usize {
    (0..centres.len())
        .min_by(|a, b| delta(centres[*a], c).total_cmp(&delta(centres[*b], c)))
        .unwrap_or(0)
}

/// Up to `k` colours that `colours` gather round, with ones that look alike merged. Always the
/// same answer for the same colours: it starts from the commonest and then each farthest.
pub(super) fn kmeans(colours: &[[f32; 3]], k: usize) -> Vec<[f32; 3]> {
    if colours.is_empty() {
        return vec![[60.0, 0.0, 0.0]];
    }
    // Start from the colour nearest the middle of them all, then each one farthest from those.
    let mean = colours.iter().fold([0.0; 3], |acc, c| {
        [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]]
    });
    let n = colours.len() as f32;
    let mean = [mean[0] / n, mean[1] / n, mean[2] / n];
    let mut centres = vec![colours[nearest(colours, mean)]];
    while centres.len() < k {
        let far = colours
            .iter()
            .copied()
            .max_by(|a, b| {
                let da = centres
                    .iter()
                    .map(|c| delta(*c, *a))
                    .fold(f32::MAX, f32::min);
                let db = centres
                    .iter()
                    .map(|c| delta(*c, *b))
                    .fold(f32::MAX, f32::min);
                da.total_cmp(&db)
            })
            .expect("there are colours");
        if centres.iter().any(|c| delta(*c, far) < 6.0) {
            break;
        }
        centres.push(far);
    }
    for _ in 0..12 {
        let mut sums = vec![([0.0_f32; 3], 0.0_f32); centres.len()];
        for c in colours {
            let i = nearest(&centres, *c);
            for (sum, value) in sums[i].0.iter_mut().zip(c) {
                *sum += value;
            }
            sums[i].1 += 1.0;
        }
        for (centre, (sum, count)) in centres.iter_mut().zip(&sums) {
            if *count > 0.0 {
                *centre = [sum[0] / count, sum[1] / count, sum[2] / count];
            }
        }
    }
    // Colours that look alike are one colour; one that almost nothing is, is dropped.
    let mut merged: Vec<[f32; 3]> = Vec::new();
    for c in centres {
        let used = colours.iter().filter(|x| delta(**x, c) < 10.0).count();
        if used == 0 && !merged.is_empty() {
            continue;
        }
        if !merged.iter().any(|m| delta(*m, c) < 10.0) {
            merged.push(c);
        }
    }
    merged
}
