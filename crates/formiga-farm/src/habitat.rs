//! The ant farm behind the workbench: a glass habitat in a wooden frame, cut through its soil,
//! with tunnels, seed chambers, roots and pebbles, and a few tiny ants about their business.
//! It is scenery and nothing more. It is drawn low in contrast and slow, so the creature on the
//! stage is always the brightest, busiest thing in the window, and nothing in it ever stands in
//! for a control.

use crate::paint::{blend, darker, lighter, rgb};
use formiga_art::{Canvas, Rgba};

/// Pixels of the habitat to a point of the window: it is drawn small and shown large, crisp.
pub const PIXEL: f32 = 3.0;

/// The time of day the habitat shows: the strip of sky above the soil, and how bright the soil
/// is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Daylight {
    Morning,
    Day,
    Dusk,
    Night,
}

impl Daylight {
    pub fn at_hour(hour: u8) -> Self {
        match hour {
            5..=8 => Self::Morning,
            9..=16 => Self::Day,
            17..=19 => Self::Dusk,
            _ => Self::Night,
        }
    }

    fn sky(self) -> (Rgba, Rgba) {
        match self {
            Self::Morning => (rgb(0xf6d9c4), rgb(0xd8e4ec)),
            Self::Day => (rgb(0xcfe3ee), rgb(0xe8f0f2)),
            Self::Dusk => (rgb(0xe8b9a0), rgb(0xb9a2c4)),
            Self::Night => (rgb(0x2c3350), rgb(0x40466a)),
        }
    }

    /// How much the soil is dimmed, from none to a little.
    fn dim(self) -> u8 {
        match self {
            Self::Night => 70,
            Self::Dusk => 25,
            _ => 0,
        }
    }
}

/// One ant's way through the tunnels.
#[derive(Clone, Debug)]
pub struct Trail {
    pub points: Vec<(i32, i32)>,
    /// How far along an ant walks a second, in pixels.
    pub speed: f32,
    /// Where along it the ant starts.
    pub offset: f32,
}

/// The habitat, drawn once for a window's size.
pub struct Habitat {
    pub canvas: Canvas,
    pub trails: Vec<Trail>,
    pub daylight: Daylight,
}

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add((y as u32).wrapping_mul(0x85eb_ca77))
        .wrapping_add(seed.wrapping_mul(0xc2b2_ae3d));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

fn noise(x: f32, seed: u32) -> f32 {
    let x0 = x.floor();
    let t = x - x0;
    let a = (hash(x0 as i32, 0, seed) % 1000) as f32 / 1000.0;
    let b = (hash(x0 as i32 + 1, 0, seed) % 1000) as f32 / 1000.0;
    let t = t * t * (3.0 - 2.0 * t);
    a + (b - a) * t
}

/// The frame's wood, the soil's bands from the top down, and the colour a tunnel is dug in.
const WOOD: [u32; 3] = [0x8a5a3c, 0xa8724c, 0x6c442e];
const STRATA: [u32; 4] = [0x7c5a44, 0x94704f, 0xa98a62, 0x8f6a52];
const TUNNEL: u32 = 0x5a3f31;

impl Habitat {
    /// A habitat `width` by `height` habitat pixels. The same size, seed and daylight always draw
    /// the same habitat.
    pub fn new(width: u32, height: u32, seed: u32, daylight: Daylight) -> Self {
        let width = width.max(64);
        let height = height.max(64);
        let mut canvas = Canvas::new(width, height);
        let (w, h) = (width as i32, height as i32);
        let frame = 4;
        let ground = (h as f32 * 0.14).round() as i32 + frame;
        let dim = daylight.dim();
        let tone = |c: Rgba| blend(c, rgb(0x1e2238), dim);

        // Sky.
        let (top, bottom) = daylight.sky();
        for y in 0..ground {
            let t = (y as f32 / ground.max(1) as f32 * 255.0) as u8;
            let row = blend(top, bottom, t);
            for x in 0..w {
                canvas.set(x, y, row);
            }
        }
        if daylight == Daylight::Night {
            for i in 0..(w / 9) {
                let x = (hash(i, 1, seed) % w as u32) as i32;
                let y = frame + (hash(i, 2, seed) % (ground - frame).max(1) as u32) as i32;
                canvas.set(x, y, rgb(0xe8e4c8));
            }
        }

        // Soil, in wavy bands.
        let bands: Vec<f32> = [0.0_f32, 0.3, 0.55, 0.8].to_vec();
        for x in 0..w {
            let edges: Vec<i32> = bands
                .iter()
                .enumerate()
                .map(|(i, at)| {
                    let wave = (noise(x as f32 / 23.0, seed + i as u32) - 0.5) * 8.0;
                    ground + ((h - ground) as f32 * at + wave) as i32
                })
                .collect();
            for y in ground..h {
                let band = edges.iter().rposition(|edge| y >= *edge).unwrap_or(0);
                let mut c = tone(rgb(STRATA[band]));
                let grain = hash(x, y, seed + 7) % 23;
                if grain == 0 {
                    c = darker(c, 18);
                } else if grain == 1 {
                    c = lighter(c, 14);
                }
                canvas.set(x, y, c);
            }
        }

        // Grass along the top of the soil.
        for x in 0..w {
            let tuft = (hash(x, 3, seed) % 4) as i32;
            for y in ground - 1 - tuft..ground + 1 {
                canvas.set(
                    x,
                    y,
                    tone(if (x + y) % 3 == 0 {
                        rgb(0x6f8f4e)
                    } else {
                        rgb(0x7fa05a)
                    }),
                );
            }
            canvas.set(x, ground + 1, tone(rgb(0x5f7a42)));
        }

        // Roots, reaching down from the grass.
        let roots = (w / 40).max(2);
        for i in 0..roots {
            let mut x = (hash(i, 11, seed) % w as u32) as f32;
            let mut y = ground as f32 + 2.0;
            let length = 12 + (hash(i, 12, seed) % 30) as i32;
            for step in 0..length {
                x += (noise(step as f32 / 5.0, seed + i as u32 * 13) - 0.5) * 1.6;
                y += 1.0;
                canvas.set(x as i32, y as i32, tone(rgb(0xc2a27a)));
                if step % 9 == 4 {
                    let side = if hash(i, step, seed).is_multiple_of(2) {
                        1
                    } else {
                        -1
                    };
                    for k in 1..4 {
                        canvas.set(x as i32 + side * k, y as i32 + k, tone(rgb(0xb4946c)));
                    }
                }
            }
        }

        // Tunnels and chambers, and the trails the ants keep to.
        let mut trails = Vec::new();
        let chambers = ((w * (h - ground)) / 5000).clamp(3, 10);
        let mut rooms: Vec<(i32, i32, i32)> = Vec::new();
        for i in 0..chambers {
            let x = frame + 10 + (hash(i, 21, seed) % (w - 2 * frame - 20).max(1) as u32) as i32;
            let y = ground + 14 + (hash(i, 22, seed) % (h - ground - 24).max(1) as u32) as i32;
            let r = 4 + (hash(i, 23, seed) % 4) as i32;
            rooms.push((x, y, r));
        }
        rooms.sort_by_key(|room| room.0);
        let entrance = (rooms.first().map_or(w / 3, |r| r.0), ground);
        let mut previous = entrance;
        for (index, &(x, y, _)) in rooms.iter().enumerate() {
            let path = dig(previous, (x, y), seed + index as u32);
            trails.push(Trail {
                points: path.clone(),
                speed: 3.0 + (hash(index as i32, 31, seed) % 3) as f32,
                offset: (hash(index as i32, 32, seed) % 100) as f32,
            });
            for &(px, py) in &path {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        canvas.set(px + dx, py + dy, tone(rgb(TUNNEL)));
                    }
                }
            }
            previous = (x, y);
        }
        for (index, &(x, y, r)) in rooms.iter().enumerate() {
            for dy in -r..=r {
                for dx in -r - 3..=r + 3 {
                    let inside = (dx * dx) as f32 / ((r + 3) * (r + 3)) as f32
                        + (dy * dy) as f32 / (r * r) as f32;
                    if inside <= 1.0 {
                        let floor = dy > r / 2;
                        canvas.set(
                            x + dx,
                            y + dy,
                            tone(if floor { rgb(0x684a39) } else { rgb(TUNNEL) }),
                        );
                    }
                }
            }
            // A store of seeds, or a crumb of leaf.
            let seeds = 2 + (hash(index as i32, 41, seed) % 4) as i32;
            for s in 0..seeds {
                let sx = x - r + 1 + s * 2;
                let sy = y + r / 2;
                let seed_color = if index % 3 == 2 {
                    rgb(0x8fae62)
                } else {
                    rgb(0xe6d3a3)
                };
                canvas.set(sx, sy, tone(seed_color));
                canvas.set(sx + 1, sy, tone(darker(seed_color, 30)));
            }
        }

        // Pebbles.
        let pebbles = (w * (h - ground)) / 900;
        for i in 0..pebbles {
            let x = (hash(i, 51, seed) % w as u32) as i32;
            let y = ground + 4 + (hash(i, 52, seed) % (h - ground - 4).max(1) as u32) as i32;
            if canvas.get(x, y) == tone(rgb(TUNNEL)) {
                continue;
            }
            let big = hash(i, 53, seed).is_multiple_of(4);
            let grey = tone(rgb(if big { 0x9c958c } else { 0x8a8178 }));
            canvas.set(x, y, grey);
            canvas.set(x + 1, y, lighter(grey, 20));
            if big {
                canvas.set(x, y + 1, darker(grey, 20));
                canvas.set(x + 1, y + 1, grey);
            }
        }

        // The glass: two faint streaks of light across it.
        for (start, width) in [(w / 5, 5), (w / 5 + 9, 2)] {
            for y in frame..h - frame {
                for k in 0..width {
                    let x = start + k + (h - y) / 2;
                    let c = canvas.get(x, y);
                    canvas.set(x, y, blend(c, rgb(0xffffff), 20));
                }
            }
        }

        // The wooden frame round it all.
        for y in 0..h {
            for x in 0..w {
                let edge = x.min(y).min(w - 1 - x).min(h - 1 - y);
                if edge < frame {
                    let grain = if (y + hash(x / 6, 61, seed) as i32 % 3) % 5 == 0 {
                        2
                    } else {
                        0
                    };
                    let c = match edge {
                        0 => rgb(WOOD[2]),
                        e if e == frame - 1 => rgb(WOOD[2]),
                        1 => rgb(WOOD[1]),
                        _ => rgb(WOOD[grain.min(2)]),
                    };
                    canvas.set(x, y, c);
                }
            }
        }

        Self {
            canvas,
            trails,
            daylight,
        }
    }

    /// Where each ant is `seconds` in, and which way it faces: along its trail and back.
    pub fn ants(&self, seconds: f32) -> Vec<(f32, f32, bool)> {
        self.trails
            .iter()
            .filter(|trail| trail.points.len() > 2)
            .map(|trail| {
                let length = trail.points.len() as f32;
                let walked = trail.offset + seconds * trail.speed;
                let lap = walked.rem_euclid(length * 2.0);
                let (at, forward) = if lap < length {
                    (lap, true)
                } else {
                    (length * 2.0 - lap, false)
                };
                let index = (at as usize).min(trail.points.len() - 1);
                let (x, y) = trail.points[index];
                let next = trail.points[(index + 1).min(trail.points.len() - 1)];
                let right = (next.0 >= x) == forward;
                (x as f32, y as f32, right)
            })
            .collect()
    }
}

/// A tunnel from `a` to `b` that wanders a little on its way.
fn dig(a: (i32, i32), b: (i32, i32), seed: u32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize).max(1);
    let mut last = a;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let wander = (noise(step as f32 / 9.0, seed) - 0.5) * 10.0 * (t * (1.0 - t) * 4.0);
        let x = a.0 as f32 + (b.0 - a.0) as f32 * t;
        let y = a.1 as f32 + (b.1 - a.1) as f32 * t + wander;
        let point = (x.round() as i32, y.round() as i32);
        // Keep every step a neighbour of the last, so an ant never jumps.
        while last != point {
            last = (
                last.0 + (point.0 - last.0).signum(),
                last.1 + (point.1 - last.1).signum(),
            );
            points.push(last);
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_habitat_is_drawn_every_time_and_its_ants_never_jump() {
        let a = Habitat::new(300, 200, 9, Daylight::Day);
        let b = Habitat::new(300, 200, 9, Daylight::Day);
        assert_eq!(a.canvas, b.canvas);
        assert!(!a.trails.is_empty());
        for trail in &a.trails {
            for pair in trail.points.windows(2) {
                let (p, q) = (pair[0], pair[1]);
                assert!((p.0 - q.0).abs() <= 1 && (p.1 - q.1).abs() <= 1);
            }
        }
        assert_eq!(a.ants(12.5), b.ants(12.5));
    }
}
