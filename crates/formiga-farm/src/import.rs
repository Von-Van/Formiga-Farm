//! Reading a picture into a design: the importer behind "From a picture".
//!
//! A picture is never pasted in or traced. It is read for what a Formiga could take from it,
//! and the answer is an ordinary design on Farm's own body plans and parts, made of the same
//! named steps as any other, which the owner then carries on shaping in the editor.
//!
//! Reading goes in four steps:
//!
//! 1. **The subject.** A picture with transparency says where its subject is. An opaque one is
//!    read from its edges: the background colours along them, and everything joined to the
//!    edges in about those colours (a gentle gradient included), is background. The largest
//!    piece left is the subject, with any holes in it filled, so a white belly or the shine in
//!    an eye stays part of it. The subject is turned to face right, as Farm's forms do, by
//!    where its eyes are, or failing that where its weight is highest.
//! 2. **The shape.** The subject's silhouette is compared with the silhouettes of Farm's own
//!    forms: every body plan and every animal preset is a starting point, and from each the
//!    proportions and parts that bring the silhouette closest are searched for, a step at a
//!    time, by drawing each one. The closest few are kept.
//! 3. **The colours.** The subject's colours are gathered into a handful, in a space where
//!    distance is how different colours look. Which is the coat, which the belly, which the
//!    markings, the accent and the horns or hooves is decided by how much of the subject each
//!    covers and where it lies. Each is set so that once Farm softens it, as every coat is
//!    softened, it shows as the picture has it.
//! 4. **The markings and the coat.** Where the second colour lies on the body says which
//!    markings it makes: long bands across the body are stripes, many small marks spots (or
//!    rosettes, with holes in them), a few large ones patches, dark rounds about the eyes eye
//!    patches, a different colour at the feet socks. How busy the colour is within the coat
//!    says whether it is smooth, furry or shaggy.
//!
//! What comes back is a few takes: the closest, the closest on another body, the closest from
//! an animal's own parts, and the closest in a plain coat.

use anyhow::{Context, Result, bail};
use formiga_core::AppearanceGenome;
use formiga_forms::{
    Coat, Design, DesignRenderer, Dimension, Form, Ink, Intent, MAX_MARKINGS, MIDDLE, Marking,
    MarkingKind, Part, PartKind, Plan, STEPS, Sculpt, Slot, Treatment, plain_face,
};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage};
use std::io::Cursor;
use std::path::Path;

/// The largest picture read, in bytes, pixels across, and pixels in all.
pub const MAX_BYTES: u64 = 24 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 6_000;
pub const MAX_PIXELS: u64 = 24_000_000;

/// The size a picture is read at: enough to see stripes and spots, small enough to be quick.
const ANALYSIS: u32 = 256;

/// The grid silhouettes are compared on.
const GRID: usize = 40;
const COARSE: usize = 8;

/// One reading of the picture, ready to start from.
#[derive(Clone, Debug)]
pub struct Take {
    pub design: Design,
    /// How alike the take's silhouette is to the picture's subject, from 0 to 100.
    pub likeness: u8,
    /// What kind of take it is: "Closest", "Another body".
    pub title: &'static str,
    /// What the take is, in a few words: "Compact four-legged, with stripes".
    pub summary: String,
}

/// The picture as it is read: small enough to be quick.
pub fn small(image: &DynamicImage) -> RgbaImage {
    image.thumbnail(ANALYSIS, ANALYSIS).to_rgba8()
}

/// Open the picture at `path` (PNG, JPEG, WebP or GIF) the right way up.
pub fn open(path: &Path) -> Result<DynamicImage> {
    let metadata = std::fs::metadata(path).context("could not read the picture")?;
    if metadata.len() > MAX_BYTES {
        bail!("the picture is larger than 24 MB");
    }
    decode(&std::fs::read(path).context("could not read the picture")?)
}

/// Open a picture's bytes, the right way up: a photo taken with the phone on its side says so
/// rather than being stored turned.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage> {
    if bytes.len() as u64 > MAX_BYTES {
        bail!("the picture is larger than 24 MB");
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("could not tell what kind of picture this is")?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Gif)
    ) {
        bail!("choose a PNG, JPEG, WebP or GIF picture");
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .context("could not open the picture")?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).context("could not open the picture")?;
    if u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS {
        bail!("the picture has too many pixels");
    }
    image.apply_orientation(orientation);
    Ok(image)
}

/// Read an already opened picture, no larger than a few hundred pixels across.
pub fn read_image(image: &RgbaImage, base: &AppearanceGenome) -> Result<Vec<Take>> {
    let picture = Picture::of(image);
    let Some(subject) = picture.subject() else {
        bail!("there is nothing in this picture to read");
    };
    let target = Silhouette::of_mask(&subject);
    let palette = Palette::read(&picture, &subject);
    let coat = palette.coat();
    let treatment = palette.treatment(&picture, &subject);
    let legs = target.suggests(Plan::CompactQuadruped) > 0.0;
    let markings = palette.markings(&picture, &subject, legs);
    let inks = palette.part_inks(&picture, &subject);
    let found = search(&target, base);
    Ok(takes(found, &coat, treatment, &markings, &inks))
}

// ---------------------------------------------------------------------------------------------
// The picture and its subject.

/// The picture in a form that is quick to read: its colours in Lab, and which pixels are opaque.
struct Picture {
    width: usize,
    height: usize,
    lab: Vec<[f32; 3]>,
    opaque: Vec<bool>,
}

/// Which pixels of the picture are the subject, turned to face right.
#[derive(Clone)]
struct Subject {
    /// The subject's box in the picture: left, top, right, bottom, inclusive.
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    cells: Vec<bool>,
    width: usize,
    /// Whether the picture was read mirrored, so the subject faces right.
    mirrored: bool,
    /// Where the eyes were seen, in the box from 0 to 1, after any mirroring.
    eyes: Option<(f32, f32)>,
}

impl Subject {
    fn has(&self, x: usize, y: usize) -> bool {
        x < self.width && self.cells.get(y * self.width + x).copied().unwrap_or(false)
    }

    fn box_width(&self) -> usize {
        self.right - self.left + 1
    }

    fn box_height(&self) -> usize {
        self.bottom - self.top + 1
    }

    /// Where a picture pixel lies in the subject's box, from 0 to 1 each way, facing right.
    fn place(&self, x: usize, y: usize) -> (f32, f32) {
        let u = (x - self.left) as f32 / self.box_width().max(1) as f32;
        let v = (y - self.top) as f32 / self.box_height().max(1) as f32;
        (if self.mirrored { 1.0 - u } else { u }, v)
    }

    /// Every pixel of the subject.
    fn pixels(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (self.top..=self.bottom)
            .flat_map(move |y| (self.left..=self.right).map(move |x| (x, y)))
            .filter(|&(x, y)| self.has(x, y))
    }

    /// How far a subject pixel is inside its edge, in pixels, up to `most`.
    fn inset(&self, x: usize, y: usize, most: usize) -> usize {
        for d in 1..=most {
            let near = [
                (d as isize, 0),
                (-(d as isize), 0),
                (0, d as isize),
                (0, -(d as isize)),
            ];
            for (dx, dy) in near {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                if nx < 0 || ny < 0 || !self.has(nx as usize, ny as usize) {
                    return d - 1;
                }
            }
        }
        most
    }
}

impl Picture {
    fn of(image: &RgbaImage) -> Self {
        let (width, height) = (image.width() as usize, image.height() as usize);
        let mut lab = Vec::with_capacity(width * height);
        let mut opaque = Vec::with_capacity(width * height);
        for pixel in image.pixels() {
            let [r, g, b, a] = pixel.0;
            // A half see-through pixel is read as if laid on white, as it would be seen.
            let over =
                |c: u8| ((u16::from(c) * u16::from(a) + 255 * (255 - u16::from(a))) / 255) as u8;
            let c = [over(r), over(g), over(b)];
            lab.push(to_lab(c));
            opaque.push(a >= 128);
        }
        Self {
            width,
            height,
            lab,
            opaque,
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Which pixels are the subject, or nothing if the picture has none to find.
    fn subject(&self) -> Option<Subject> {
        let total = self.width * self.height;
        if total == 0 {
            return None;
        }
        let see_through = self.opaque.iter().filter(|o| !**o).count();
        let clear = see_through > total / 50;
        let mut cells = if clear {
            self.opaque.clone()
        } else {
            self.foreground()
        };
        let count = cells.iter().filter(|c| **c).count();
        if clear && count < total / 1000 {
            // A clear picture with nothing, or next to nothing, drawn on it.
            return None;
        }
        if !clear && (count < total / 200 || count > total * 97 / 100) {
            // Nothing stood apart from the background: the subject fills the picture.
            cells = vec![true; total];
        }
        let cells = largest_piece(&cells, self.width, self.height);
        let cells = fill_holes(&cells, self.width, self.height);
        let cells = trim(&cells, self.width, self.height);
        let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                if cells[self.index(x, y)] {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }
        if left > right {
            return None;
        }
        let mut subject = Subject {
            left,
            top,
            right,
            bottom,
            cells,
            width: self.width,
            mirrored: false,
            eyes: None,
        };
        let eyes = self.eyes(&subject);
        // The head is where the weight is highest, when that is plain; otherwise wherever the
        // eyes are; otherwise the slightest lean decides.
        let lean = self.lean(&subject);
        let facing_left = match eyes {
            _ if lean.abs() > 0.12 => lean > 0.0,
            Some((u, _)) => u < 0.5,
            None => lean > 0.0,
        };
        // Eyes on the wrong side were something else.
        let eyes = eyes.filter(|(u, _)| (*u < 0.5) == facing_left);
        subject.mirrored = facing_left;
        subject.eyes = eyes.map(|(u, v)| if facing_left { (1.0 - u, v) } else { (u, v) });
        Some(subject)
    }

    /// Everything that is not background. The background is whatever colours run along the
    /// picture's edges, followed in from them while the colour stays near one of those. A soft
    /// gradient runs along the edges too, so each of its shades is one of those colours.
    fn foreground(&self) -> Vec<bool> {
        let (w, h) = (self.width, self.height);
        let mut border = Vec::new();
        for x in 0..w {
            border.push(self.index(x, 0));
            border.push(self.index(x, h - 1));
        }
        for y in 0..h {
            border.push(self.index(0, y));
            border.push(self.index(w - 1, y));
        }
        // The background colours, gathered from the edge: a gradient gives several.
        let mut backgrounds: Vec<([f32; 3], usize)> = Vec::new();
        for &i in &border {
            let c = self.lab[i];
            match backgrounds.iter_mut().find(|(b, _)| delta(*b, c) < 8.0) {
                Some((b, n)) => {
                    *n += 1;
                    let k = 1.0 / *n as f32;
                    for ch in 0..3 {
                        b[ch] += (c[ch] - b[ch]) * k;
                    }
                }
                None => backgrounds.push((c, 1)),
            }
        }
        let enough = (border.len() / 60).max(2);
        let backgrounds: Vec<[f32; 3]> = backgrounds
            .iter()
            .filter(|(_, n)| *n >= enough)
            .map(|(c, _)| *c)
            .collect();
        let near_background = |c: [f32; 3]| backgrounds.iter().any(|b| delta(*b, c) < 12.0);
        let mut background = vec![false; w * h];
        let mut stack = Vec::new();
        for &i in &border {
            if !background[i] && near_background(self.lab[i]) {
                background[i] = true;
                stack.push(i);
            }
        }
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            let neighbours = [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ];
            for (nx, ny) in neighbours {
                if nx >= w || ny >= h {
                    continue;
                }
                let n = self.index(nx, ny);
                if !background[n] && near_background(self.lab[n]) {
                    background[n] = true;
                    stack.push(n);
                }
            }
        }
        background.iter().map(|b| !b).collect()
    }

    /// Where the eyes are, in the subject's box from 0 to 1: the middle of the dark rounds well
    /// inside the upper part of the subject, if there are any.
    fn eyes(&self, subject: &Subject) -> Option<(f32, f32)> {
        let (bw, bh) = (subject.box_width(), subject.box_height());
        let mut dark = vec![false; self.width * self.height];
        let mut any = 0;
        for (x, y) in subject.pixels() {
            let (_, v) = (
                (x - subject.left) as f32 / bw as f32,
                (y - subject.top) as f32 / bh as f32,
            );
            if v < 0.7 && self.lab[self.index(x, y)][0] < 28.0 && subject.inset(x, y, 3) >= 2 {
                dark[self.index(x, y)] = true;
                any += 1;
            }
        }
        if any == 0 {
            return None;
        }
        // Eyes are small rounds: a dark piece that is neither a speck nor most of the subject,
        // about as tall as it is wide.
        let area = subject.pixels().count() as f32;
        let mut best: Option<(f32, f32, f32)> = None;
        for piece in pieces(&dark, self.width, self.height) {
            let n = piece.len() as f32;
            if n < 2.0 || n > area * 0.06 {
                continue;
            }
            let (mut l, mut t, mut r, mut b) = (usize::MAX, usize::MAX, 0, 0);
            let (mut sx, mut sy) = (0.0, 0.0);
            for &i in &piece {
                let (x, y) = (i % self.width, i / self.width);
                l = l.min(x);
                t = t.min(y);
                r = r.max(x);
                b = b.max(y);
                sx += x as f32;
                sy += y as f32;
            }
            let (pw, ph) = ((r - l + 1) as f32, (b - t + 1) as f32);
            let round = pw.min(ph) / pw.max(ph);
            let fill = n / (pw * ph);
            if round < 0.45 || fill < 0.5 {
                continue;
            }
            let u = (sx / n - subject.left as f32) / bw as f32;
            let v = (sy / n - subject.top as f32) / bh as f32;
            let score = n * round;
            if best.is_none_or(|(s, _, _)| score > s) {
                best = Some((score, u, v));
            }
        }
        best.map(|(_, u, v)| (u, v))
    }

    /// Which of the subject's pixels are drawn lines: dark, greyish, and too thin to hold a
    /// square five pixels across.
    fn lines(&self, subject: &Subject) -> Vec<bool> {
        let (w, h) = (self.width, self.height);
        let dark: Vec<bool> = (0..w * h)
            .map(|i| {
                let c = self.lab[i];
                subject.has(i % w, i / w) && c[0] < 35.0 && chroma(c) < 18.0
            })
            .collect();
        let thick = dilate(&erode(&dark, w, h, 2), w, h, 2);
        dark.iter().zip(&thick).map(|(d, t)| *d && !t).collect()
    }

    /// How much higher the subject's weight is on its left than its right, from -1 to 1: most
    /// creatures carry their head above their tail.
    fn lean(&self, subject: &Subject) -> f32 {
        let (bw, bh) = (subject.box_width() as f32, subject.box_height() as f32);
        let (mut left, mut right) = (0.0, 0.0);
        for (x, y) in subject.pixels() {
            let (u, v) = (
                (x - subject.left) as f32 / bw,
                (y - subject.top) as f32 / bh,
            );
            if v < 0.45 {
                if u < 0.5 {
                    left += 1.0 - v;
                } else {
                    right += 1.0 - v;
                }
            }
        }
        if left + right > 0.0 {
            (left - right) / (left + right)
        } else {
            0.0
        }
    }
}

/// Every piece of `cells`, joined side to side, as lists of indices.
fn pieces(cells: &[bool], width: usize, height: usize) -> Vec<Vec<usize>> {
    let mut seen = vec![false; cells.len()];
    let mut found = Vec::new();
    for start in 0..cells.len() {
        if !cells[start] || seen[start] {
            continue;
        }
        let mut piece = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            piece.push(i);
            let (x, y) = (i % width, i / width);
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= width || ny >= height {
                    continue;
                }
                let n = ny * width + nx;
                if cells[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        found.push(piece);
    }
    found
}

/// Only the largest piece of `cells`.
fn largest_piece(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let mut out = vec![false; cells.len()];
    if let Some(piece) = pieces(cells, width, height)
        .into_iter()
        .max_by_key(Vec::len)
    {
        for i in piece {
            out[i] = true;
        }
    }
    out
}

/// `cells` without whatever thin thing runs off the picture's edge, as the branch a bird sits
/// on or the line of the ground does, keeping thin parts of the subject itself, as its tail
/// or legs.
fn trim(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let count = cells.iter().filter(|c| **c).count();
    let radius = ((count as f32).sqrt() / 25.0).round().max(1.0) as usize;
    let thick = dilate(&erode(cells, width, height, radius), width, height, radius);
    let thick: Vec<bool> = thick.iter().zip(cells).map(|(t, c)| *t && *c).collect();
    if thick.iter().filter(|c| **c).count() < count / 3 {
        // The subject is all thin parts: keep it as it is.
        return cells.to_vec();
    }
    let mut out = largest_piece(&thick, width, height);
    let thin: Vec<bool> = cells.iter().zip(&thick).map(|(c, t)| *c && !t).collect();
    for piece in pieces(&thin, width, height) {
        let touches_edge = piece.iter().any(|&i| {
            let (x, y) = (i % width, i / width);
            x == 0 || y == 0 || x + 1 == width || y + 1 == height
        });
        if !touches_edge {
            for i in piece {
                out[i] = true;
            }
        }
    }
    largest_piece(&out, width, height)
}

/// `cells` with everything within `radius` of an empty cell (or the edge) emptied.
fn erode(cells: &[bool], width: usize, height: usize, radius: usize) -> Vec<bool> {
    let r = radius as isize;
    let across: Vec<bool> = (0..cells.len())
        .map(|i| {
            let (x, y) = ((i % width) as isize, i / width);
            (-r..=r).all(|d| {
                let nx = x + d;
                nx >= 0 && (nx as usize) < width && cells[y * width + nx as usize]
            })
        })
        .collect();
    (0..cells.len())
        .map(|i| {
            let (x, y) = (i % width, (i / width) as isize);
            (-r..=r).all(|d| {
                let ny = y + d;
                ny >= 0 && (ny as usize) < height && across[ny as usize * width + x]
            })
        })
        .collect()
}

/// `cells` with everything within `radius` of a filled cell filled.
fn dilate(cells: &[bool], width: usize, height: usize, radius: usize) -> Vec<bool> {
    let outside: Vec<bool> = cells.iter().map(|c| !c).collect();
    // Dilating is eroding the outside; the picture's edge counts as inside here.
    let r = radius as isize;
    let across: Vec<bool> = (0..cells.len())
        .map(|i| {
            let (x, y) = ((i % width) as isize, i / width);
            (-r..=r).all(|d| {
                let nx = x + d;
                nx < 0 || nx as usize >= width || outside[y * width + nx as usize]
            })
        })
        .collect();
    (0..cells.len())
        .map(|i| {
            let (x, y) = (i % width, (i / width) as isize);
            !(-r..=r).all(|d| {
                let ny = y + d;
                ny < 0 || ny as usize >= height || across[ny as usize * width + x]
            })
        })
        .collect()
}

/// `cells` with every hole that does not reach the edge filled in.
fn fill_holes(cells: &[bool], width: usize, height: usize) -> Vec<bool> {
    let outside: Vec<bool> = cells.iter().map(|c| !c).collect();
    let mut out = cells.to_vec();
    for piece in pieces(&outside, width, height) {
        let touches_edge = piece.iter().any(|&i| {
            let (x, y) = (i % width, i / width);
            x == 0 || y == 0 || x + 1 == width || y + 1 == height
        });
        if !touches_edge {
            for i in piece {
                out[i] = true;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Silhouettes, and the search for the form whose silhouette is closest.

/// A silhouette fitted into a square grid with its proportions kept, and those proportions.
struct Silhouette {
    cells: Vec<f32>,
    /// The same, read broadly: each cell an eighth of the way across.
    coarse: Vec<f32>,
    aspect: f32,
}

impl Silhouette {
    fn of_mask(subject: &Subject) -> Self {
        Self::fit(subject.box_width(), subject.box_height(), |x, y| {
            let px = if subject.mirrored {
                subject.right - x
            } else {
                subject.left + x
            };
            subject.has(px, subject.top + y)
        })
    }

    fn of_canvas(canvas: &formiga_art::Canvas) -> Option<Self> {
        let (w, h) = (canvas.width() as i32, canvas.height() as i32);
        let (mut left, mut top, mut right, mut bottom) = (w, h, -1, -1);
        for y in 0..h {
            for x in 0..w {
                if canvas.get(x, y).a > 0 {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }
        if right < left {
            return None;
        }
        Some(Self::fit(
            (right - left + 1) as usize,
            (bottom - top + 1) as usize,
            |x, y| canvas.get(left + x as i32, top + y as i32).a > 0,
        ))
    }

    /// `width` by `height` cells read by `has`, stretched to fill the grid, each grid cell
    /// holding how much of it is covered. Stretched rather than fitted, so a long lizard and
    /// Farm's shorter crawler are compared for where their parts are, and their proportions
    /// are weighed apart.
    fn fit(width: usize, height: usize, has: impl Fn(usize, usize) -> bool) -> Self {
        let (sx_scale, sy_scale) = (width as f32 / GRID as f32, height as f32 / GRID as f32);
        let mut cells = vec![0.0; GRID * GRID];
        const SAMPLES: usize = 3;
        for gy in 0..GRID {
            for gx in 0..GRID {
                let mut covered = 0;
                for sy in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        let fx = (gx as f32 + (sx as f32 + 0.5) / SAMPLES as f32) * sx_scale;
                        let fy = (gy as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * sy_scale;
                        if (fx as usize) < width
                            && (fy as usize) < height
                            && has(fx as usize, fy as usize)
                        {
                            covered += 1;
                        }
                    }
                }
                cells[gy * GRID + gx] = covered as f32 / (SAMPLES * SAMPLES) as f32;
            }
        }
        let mut coarse = vec![0.0; COARSE * COARSE];
        let step = GRID / COARSE;
        for (i, cell) in cells.iter().enumerate() {
            let (gx, gy) = (i % GRID, i / GRID);
            coarse[(gy / step) * COARSE + gx / step] += cell / (step * step) as f32;
        }
        Self {
            cells,
            coarse,
            aspect: width as f32 / height.max(1) as f32,
        }
    }

    /// What the silhouette is like, whatever style it is drawn in: how much room there is under
    /// its middle, how far its front stands above its middle, how much of it is in its upper
    /// half, and how much of its box it fills.
    fn cues(&self) -> [f32; 5] {
        let filled = |gx: usize, gy: usize| self.cells[gy * GRID + gx] >= 0.5;
        let middle = GRID * 3 / 10..GRID * 7 / 10;
        let low = GRID * 6 / 10..GRID;
        let mut under = 0.0;
        for gx in middle.clone() {
            under += low.clone().filter(|&gy| !filled(gx, gy)).count() as f32 / low.len() as f32;
        }
        let under = under / middle.len() as f32;
        let top = |columns: std::ops::Range<usize>| {
            let n = columns.len() as f32;
            columns
                .map(|gx| (0..GRID).find(|&gy| filled(gx, gy)).unwrap_or(GRID) as f32)
                .sum::<f32>()
                / n
                / GRID as f32
        };
        let front = top(GRID * 7 / 10..GRID);
        let raised = (top(middle) - front).max(0.0);
        let total: f32 = self.cells.iter().sum::<f32>().max(1.0);
        let upper: f32 = self.cells[..GRID * GRID / 2].iter().sum::<f32>() / total;
        // Feet: the separate pieces the silhouette stands on.
        let floor = GRID * 92 / 100;
        let mut feet = 0;
        let mut run = 0;
        for gx in 0..=GRID {
            if gx < GRID && filled(gx, floor) {
                run += 1;
            } else {
                if run >= 2 {
                    feet += 1;
                }
                run = 0;
            }
        }
        [
            under,
            raised,
            upper,
            total / (GRID * GRID) as f32,
            feet as f32,
        ]
    }

    /// How strongly the silhouette's build suggests `plan`, from 0 to 1, whatever its outline:
    /// Farm's creatures are drawn with large heads and short legs, so a picture's outline alone
    /// would always lean toward the stoutest bodies. Something standing on two or more feet with
    /// room under it is four-legged; something very long is a crawler; something taller than
    /// it is wide stands upright or perches; something with no feet to stand on swims if it is
    /// full and round, and otherwise perches, as a bird does.
    fn suggests(&self, plan: Plan) -> f32 {
        let [under, _, _, fill, feet] = self.cues();
        let legs = feet >= 3.0 || (feet >= 2.0 && under >= 0.3);
        match plan {
            _ if self.aspect >= 2.8 => match plan {
                Plan::Crawler => 1.0,
                Plan::Floater => 0.5,
                _ => 0.0,
            },
            Plan::CompactQuadruped | Plan::LargeQuadruped | Plan::TallQuadruped if legs => 1.0,
            _ if legs => 0.0,
            Plan::Upright | Plan::Percher if self.aspect <= 0.85 => 1.0,
            _ if self.aspect <= 0.85 => 0.0,
            Plan::Floater if fill >= 0.62 => 1.0,
            Plan::Crawler if fill >= 0.62 => 0.5,
            Plan::Percher if fill < 0.62 => 1.0,
            Plan::Floater => 0.5,
            _ => 0.0,
        }
    }

    /// How alike two silhouettes are, from 0 to 1: how much they overlap, closely and broadly,
    /// less a little for proportions that differ.
    fn likeness(&self, other: &Self) -> f32 {
        let overlap = |a: &[f32], b: &[f32]| {
            let (mut both, mut either) = (0.0, 0.0);
            for (a, b) in a.iter().zip(b) {
                both += a.min(*b);
                either += a.max(*b);
            }
            if either > 0.0 { both / either } else { 0.0 }
        };
        let close = overlap(&self.cells, &other.cells);
        let broad = overlap(&self.coarse, &other.coarse);
        let proportions = (self.aspect / other.aspect.max(0.01)).ln().abs().min(1.2);
        (0.6 * close + 0.4 * broad - 0.15 * proportions).clamp(0.0, 1.0)
    }
}

/// A form found by the search, and how alike it is.
#[derive(Clone)]
struct Found {
    sculpt: Sculpt,
    likeness: f32,
    /// The animal it was started from, if it was.
    animal: Option<&'static str>,
}

/// What a form costs in parts: each part that changes its outline has to bring it this much
/// closer to the picture to be worth fitting, so a form is not hung with ears and horns that
/// only nudge its outline.
fn parts_cost(sculpt: &Sculpt) -> f32 {
    0.015
        * SHAPING
            .iter()
            .filter(|slot| sculpt.part(**slot).is_some())
            .count() as f32
}

fn likeness(sculpt: &Sculpt, target: &Silhouette, base: &AppearanceGenome) -> f32 {
    let design = Design {
        form: Form::Sculpted {
            sculpt: sculpt.clone(),
        },
        face: plain_face(),
    };
    let body = DesignRenderer::body_frame(&design, base, Intent::Idle.clip(), 0, true);
    Silhouette::of_canvas(&body.canvas).map_or(0.0, |s| target.likeness(&s))
}

/// The slots whose parts change a silhouette, which the search tries parts in.
const SHAPING: [Slot; 10] = [
    Slot::Ears,
    Slot::Snout,
    Slot::Horns,
    Slot::Mane,
    Slot::Tail,
    Slot::Fins,
    Slot::Back,
    Slot::Wings,
    Slot::Tusks,
    Slot::Shell,
];

/// The colour a part newly fitted in `kind`'s slot is drawn in.
fn ink_for(kind: PartKind) -> Ink {
    use PartKind as K;
    match kind {
        K::SnoutBeak | K::SnoutHookedBeak | K::FeetHooves | K::FeetTalons => Ink::Feature,
        K::SnoutMuzzle | K::SnoutLong | K::SnoutButton => Ink::Underside,
        _ => match kind.slot() {
            Slot::Horns | Slot::Tusks => Ink::Feature,
            Slot::Mane | Slot::Shell => Ink::Secondary,
            _ => Ink::Primary,
        },
    }
}

/// Every form one step from `sculpt`: a proportion moved, a part fitted, swapped, taken off or
/// resized. With `parts` false, only the proportions.
fn neighbours(sculpt: &Sculpt, parts: bool, worn: &[(Plan, PartKind)]) -> Vec<Sculpt> {
    let mut out = Vec::new();
    for dimension in Dimension::ALL {
        if !sculpt.plan.uses(dimension) {
            continue;
        }
        let value = sculpt.shape.get(dimension);
        for step in [-3_i8, -1, 1, 3] {
            let next = i16::from(value) + i16::from(step);
            if (0..=i16::from(STEPS)).contains(&next) {
                let mut s = sculpt.clone();
                s.shape.set(dimension, next as u8);
                out.push(s);
            }
        }
    }
    if !parts {
        return out;
    }
    for slot in SHAPING {
        if !sculpt.plan.has(slot) {
            continue;
        }
        let current = sculpt.part(slot).copied();
        // A snout hardly shows in an outline, and a face without one is not what any picture
        // asked for.
        if current.is_some() && slot != Slot::Snout {
            let mut s = sculpt.clone();
            s.remove(slot);
            out.push(s);
        }
        for kind in slot.kinds() {
            if current.is_some_and(|part| part.kind == kind) || !worn.contains(&(sculpt.plan, kind))
            {
                continue;
            }
            let mut s = sculpt.clone();
            let size = current.map_or(MIDDLE, |part| part.size);
            s.fit(Part::new(kind, ink_for(kind)).sized(size));
            out.push(s);
        }
        if let Some(part) = current {
            for step in [-2_i8, 2] {
                let size = i16::from(part.size) + i16::from(step);
                if (0..=i16::from(STEPS)).contains(&size) {
                    let mut s = sculpt.clone();
                    s.fit(part.sized(size as u8));
                    out.push(s);
                }
            }
        }
    }
    out.into_iter().map(|s| s.normalized()).collect()
}

/// Step from `start` to whichever neighbour is closest, until none is closer.
fn climb(
    start: Found,
    target: &Silhouette,
    base: &AppearanceGenome,
    parts: bool,
    worn: &[(Plan, PartKind)],
    rounds: usize,
) -> Found {
    let mut here = start;
    for _ in 0..rounds {
        let best = neighbours(&here.sculpt, parts, worn)
            .into_iter()
            .map(|sculpt| {
                let likeness = likeness(&sculpt, target, base);
                (sculpt, likeness)
            })
            .max_by(|a, b| (a.1 - parts_cost(&a.0)).total_cmp(&(b.1 - parts_cost(&b.0))));
        let worth = |sculpt: &Sculpt, likeness: f32| likeness - parts_cost(sculpt);
        match best {
            Some((sculpt, likeness))
                if worth(&sculpt, likeness) > worth(&here.sculpt, here.likeness) + 0.002 =>
            {
                here = Found {
                    sculpt,
                    likeness,
                    animal: here.animal,
                };
            }
            _ => break,
        }
    }
    here
}

/// Every starting point: each plan's own starter, and each animal.
fn starts() -> Vec<(Sculpt, Option<&'static str>)> {
    let mut starts: Vec<(Sculpt, Option<&'static str>)> = Plan::ALL
        .into_iter()
        .map(|plan| (Sculpt::starter(plan), None))
        .collect();
    for preset in crate::presets::animals() {
        if let Form::Sculpted { sculpt } = preset.design.form {
            starts.push((sculpt, Some(preset.name)));
        }
    }
    starts
}

/// Search every starting point for the forms closest to `target`, closest first.
fn search(target: &Silhouette, base: &AppearanceGenome) -> Vec<Found> {
    let starts = starts();
    // The parts each plan is seen wearing, on its starter or an animal: a cat is not offered a
    // fish's tail, nor a lizard an elephant's ears.
    let mut worn: Vec<(Plan, PartKind)> = Vec::new();
    for (sculpt, _) in &starts {
        for part in &sculpt.parts {
            if !worn.contains(&(sculpt.plan, part.kind)) {
                worn.push((sculpt.plan, part.kind));
            }
        }
    }
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 8);
    // First, every start with its proportions fitted.
    let rough: Vec<Found> = in_parallel(starts, threads, |(sculpt, animal)| {
        let sculpt = sculpt.normalized();
        let found = Found {
            likeness: likeness(&sculpt, target, base),
            sculpt,
            animal,
        };
        climb(found, target, base, false, &worn, 3)
    });
    // Then the closest on each plan, and the closest few animals, with their parts tried too.
    let mut chosen: Vec<Found> = Vec::new();
    let mut by_likeness = rough.clone();
    by_likeness.sort_by(|a, b| b.likeness.total_cmp(&a.likeness));
    for plan in Plan::ALL {
        if let Some(best) = by_likeness
            .iter()
            .find(|f| f.sculpt.plan == plan && f.animal.is_none())
        {
            chosen.push(best.clone());
        }
    }
    let fit = |f: &Found| f.likeness + 0.15 * target.suggests(f.sculpt.plan);
    by_likeness.sort_by(|a, b| fit(b).total_cmp(&fit(a)));
    chosen.extend(
        by_likeness
            .iter()
            .filter(|f| f.animal.is_some())
            .take(4)
            .cloned(),
    );
    let mut found = in_parallel(chosen, threads, |found| {
        climb(found, target, base, true, &worn, 8)
    });
    // Closest first, leaning toward the bodies the picture's build suggests.
    found.sort_by(|a, b| fit(b).total_cmp(&fit(a)));
    found
}

/// `work` done on every item, across `threads` threads, in the items' order.
fn in_parallel<T: Send, U: Send>(
    items: Vec<T>,
    threads: usize,
    work: impl Fn(T) -> U + Sync,
) -> Vec<U> {
    let count = items.len();
    let queue = std::sync::Mutex::new(items.into_iter().enumerate().collect::<Vec<_>>());
    let results = std::sync::Mutex::new((0..count).map(|_| None).collect::<Vec<Option<U>>>());
    std::thread::scope(|scope| {
        for _ in 0..threads.min(count.max(1)) {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("the queue is never poisoned").pop();
                    let Some((index, item)) = next else { break };
                    let result = work(item);
                    results.lock().expect("the results are never poisoned")[index] = Some(result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("the results are never poisoned")
        .into_iter()
        .map(|r| r.expect("every item is worked"))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Colours.

/// One colour of the subject: how much of it there is and where it lies.
#[derive(Clone, Copy, Debug)]
struct Swatch {
    lab: [f32; 3],
    rgb: [u8; 3],
    /// The share of the subject it covers.
    share: f32,
    /// Its middle, in the subject's box from 0 to 1, facing right.
    at: (f32, f32),
    /// The share of it in the lower middle of the body, where a belly is.
    belly: f32,
    /// The share of it in the lowest rows, where feet are.
    feet: f32,
}

/// The subject's colours, and which of them is which.
struct Palette {
    swatches: Vec<Swatch>,
    /// For every subject pixel, which swatch it is.
    which: Vec<usize>,
    primary: usize,
    underside: Option<usize>,
    secondary: Option<usize>,
    accent: Option<usize>,
    feature: Option<usize>,
}

impl Palette {
    fn read(picture: &Picture, subject: &Subject) -> Self {
        let pixels: Vec<(usize, usize)> = subject.pixels().collect();
        let colours: Vec<[f32; 3]> = pixels
            .iter()
            .map(|&(x, y)| picture.lab[picture.index(x, y)])
            .collect();
        // Outlines say nothing about colour: dark lines too thin to be markings of their own.
        // Dark areas, a panda's legs or a zebra's stripes, are colour like any other.
        let lines = picture.lines(subject);
        let counted: Vec<bool> = pixels
            .iter()
            .map(|&(x, y)| !lines[picture.index(x, y)])
            .collect();
        let centres = kmeans(
            &colours
                .iter()
                .zip(&counted)
                .filter(|(_, keep)| **keep)
                .map(|(c, _)| *c)
                .collect::<Vec<_>>(),
            6,
        );
        let mut swatches: Vec<Swatch> = centres
            .iter()
            .map(|lab| Swatch {
                lab: *lab,
                rgb: from_lab(*lab),
                share: 0.0,
                at: (0.0, 0.0),
                belly: 0.0,
                feet: 0.0,
            })
            .collect();
        let mut which = vec![usize::MAX; picture.width * picture.height];
        let mut sums = vec![(0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32); swatches.len()];
        for (i, &(x, y)) in pixels.iter().enumerate() {
            if !counted[i] {
                continue;
            }
            let nearest = nearest(&centres, colours[i]);
            which[picture.index(x, y)] = nearest;
            let (u, v) = subject.place(x, y);
            let s = &mut sums[nearest];
            s.0 += 1.0;
            s.1 += u;
            s.2 += v;
            if (0.2..0.8).contains(&u) && v > 0.5 {
                s.3 += 1.0;
            }
            if v > 0.88 {
                s.4 += 1.0;
            }
        }
        let total = sums.iter().map(|s| s.0).sum::<f32>().max(1.0);
        for (swatch, s) in swatches.iter_mut().zip(&sums) {
            let n = s.0.max(1.0);
            swatch.share = s.0 / total;
            swatch.at = (s.1 / n, s.2 / n);
            swatch.belly = s.3 / n;
            swatch.feet = s.4 / n;
        }
        // The coat is the colour covering most of the body's middle.
        let mut core = vec![0.0_f32; swatches.len()];
        for &(x, y) in &pixels {
            let w = which[picture.index(x, y)];
            if w == usize::MAX {
                continue;
            }
            let (u, v) = subject.place(x, y);
            if (0.15..0.85).contains(&u) && (0.15..0.85).contains(&v) {
                core[w] += 1.0;
            }
        }
        let primary = (0..swatches.len())
            .max_by(|a, b| core[*a].total_cmp(&core[*b]))
            .unwrap_or(0);
        let p = swatches[primary];
        let distinct = |a: &Swatch, b: &Swatch| delta(a.lab, b.lab) > 12.0;
        let underside = (0..swatches.len())
            .filter(|&i| i != primary)
            .filter(|&i| {
                let s = swatches[i];
                s.share > 0.04
                    && distinct(&s, &p)
                    && s.lab[0] > p.lab[0] + 8.0
                    && (s.belly > 0.35 || s.at.1 > p.at.1 + 0.06)
            })
            .max_by(|a, b| swatches[*a].share.total_cmp(&swatches[*b].share));
        let secondary = (0..swatches.len())
            .filter(|&i| i != primary && Some(i) != underside)
            .filter(|&i| swatches[i].share > 0.03 && distinct(&swatches[i], &p))
            .max_by(|a, b| swatches[*a].share.total_cmp(&swatches[*b].share));
        let taken = |i: usize| i == primary || Some(i) == underside || Some(i) == secondary;
        let accent = (0..swatches.len())
            .filter(|&i| !taken(i))
            .filter(|&i| {
                let s = swatches[i];
                s.share > 0.004 && chroma(s.lab) > 22.0 && distinct(&s, &p)
            })
            .max_by(|a, b| chroma(swatches[*a].lab).total_cmp(&chroma(swatches[*b].lab)));
        let feature = (0..swatches.len())
            .filter(|&i| !taken(i) && Some(i) != accent)
            .filter(|&i| swatches[i].share > 0.004 && swatches[i].feet > 0.4)
            .max_by(|a, b| swatches[*a].feet.total_cmp(&swatches[*b].feet));
        Self {
            swatches,
            which,
            primary,
            underside,
            secondary,
            accent,
            feature,
        }
    }

    /// The coat these colours make, set so that each shows as the picture has it once Farm
    /// softens it.
    fn coat(&self) -> Coat {
        let p = self.swatches[self.primary];
        let primary = p.rgb;
        let underside = self.underside.map_or_else(
            || from_lab([(p.lab[0] + 22.0).min(96.0), p.lab[1] * 0.4, p.lab[2] * 0.4]),
            |i| self.swatches[i].rgb,
        );
        let secondary = self.secondary.map_or_else(
            || from_lab([(p.lab[0] - 22.0).max(8.0), p.lab[1], p.lab[2]]),
            |i| self.swatches[i].rgb,
        );
        let accent = self
            .accent
            .map_or([0xe8, 0x9c, 0x9c], |i| self.swatches[i].rgb);
        let feature = self.feature.map_or_else(
            || {
                if p.lab[0] > 60.0 {
                    from_lab([(p.lab[0] - 45.0).max(18.0), p.lab[1] * 0.3, p.lab[2] * 0.3])
                } else {
                    [0xf0, 0xe6, 0xd6]
                }
            },
            |i| self.swatches[i].rgb,
        );
        // How far the belly colour reaches up the front.
        let reach = self.underside.map_or(2, |i| {
            let s = self.swatches[i];
            ((s.share * 18.0 + s.belly * 4.0).round() as u8).clamp(2, STEPS)
        });
        Coat {
            treatment: Treatment::Fur,
            primary: unsoften(primary),
            secondary: unsoften(secondary),
            underside: unsoften(underside),
            accent: unsoften(accent),
            feature: unsoften(feature),
            eyes: Coat::default().eyes,
            underside_reach: reach,
        }
    }

    /// How busy the coat's own colour is from pixel to pixel: smooth skin is calm, fur is not,
    /// and long fur is busier still.
    fn treatment(&self, picture: &Picture, subject: &Subject) -> Treatment {
        let mut busy = 0.0;
        let mut counted = 0.0;
        for (x, y) in subject.pixels() {
            if subject.inset(x, y, 3) < 3 || self.which[picture.index(x, y)] != self.primary {
                continue;
            }
            let here = picture.lab[picture.index(x, y)][0];
            let mut most: f32 = 0.0;
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if self.which[picture.index(nx, ny)] == self.primary {
                    most = most.max((picture.lab[picture.index(nx, ny)][0] - here).abs());
                }
            }
            busy += most;
            counted += 1.0;
        }
        let busy = if counted > 0.0 { busy / counted } else { 0.0 };
        if busy < 1.2 {
            Treatment::Smooth
        } else if busy > 6.0 {
            Treatment::Shaggy
        } else {
            Treatment::Fur
        }
    }

    /// The swatch most of the subject is in where `region` says (in the subject's box from 0 to
    /// 1, facing right), and how much of it that is; drawn lines aside.
    fn dominant(
        &self,
        picture: &Picture,
        subject: &Subject,
        region: impl Fn(f32, f32) -> bool,
    ) -> Option<(usize, f32)> {
        let mut counts = vec![0_usize; self.swatches.len()];
        for (x, y) in subject.pixels() {
            let (u, v) = subject.place(x, y);
            let which = self.which[picture.index(x, y)];
            if which != usize::MAX && region(u, v) {
                counts[which] += 1;
            }
        }
        let total: usize = counts.iter().sum();
        if total < 8 {
            return None;
        }
        let (best, count) = counts.iter().enumerate().max_by_key(|(_, c)| **c)?;
        Some((best, *count as f32 / total as f32))
    }

    /// The coat colour each slot's part is drawn in, from the colour of the picture where that
    /// part would be: a panda's ears and feet black, a bird's beak and feet orange.
    fn part_inks(&self, picture: &Picture, subject: &Subject) -> Vec<(Slot, Ink)> {
        let roles = [
            (Some(self.primary), Ink::Primary),
            (self.underside, Ink::Underside),
            (self.secondary, Ink::Secondary),
            (self.accent, Ink::Accent),
            (self.feature, Ink::Feature),
        ];
        let ink_of = |swatch: usize| {
            roles
                .iter()
                .filter_map(|(role, ink)| role.map(|r| (r, *ink)))
                .min_by(|a, b| {
                    let lab = self.swatches[swatch].lab;
                    delta(self.swatches[a.0].lab, lab)
                        .total_cmp(&delta(self.swatches[b.0].lab, lab))
                })
                .map_or(Ink::Primary, |(_, ink)| ink)
        };
        type Region = fn(f32, f32) -> bool;
        let regions: [(Slot, Region); 9] = [
            (Slot::Ears, |u, v| u > 0.55 && v < 0.12),
            (Slot::Feet, |_, v| v > 0.88),
            (Slot::Tail, |u, v| u < 0.18 && (0.1..0.8).contains(&v)),
            (Slot::Snout, |u, v| u > 0.85 && (0.15..0.6).contains(&v)),
            (Slot::Horns, |u, v| u > 0.5 && v < 0.12),
            (Slot::Mane, |u, v| {
                (0.5..0.85).contains(&u) && (0.1..0.45).contains(&v)
            }),
            (Slot::Back, |u, v| (0.3..0.7).contains(&u) && v < 0.2),
            (Slot::Wings, |u, v| {
                (0.3..0.7).contains(&u) && (0.3..0.6).contains(&v)
            }),
            (Slot::Shell, |u, v| {
                (0.25..0.75).contains(&u) && (0.2..0.6).contains(&v)
            }),
        ];
        regions
            .into_iter()
            .filter_map(|(slot, region)| {
                let (swatch, share) = self.dominant(picture, subject, region)?;
                (share >= 0.4).then(|| (slot, ink_of(swatch)))
            })
            .collect()
    }

    /// The markings the coat's other colours make: socks, dark rounds about the eyes, and
    /// whatever the second colour makes on the body.
    fn markings(&self, picture: &Picture, subject: &Subject, legs: bool) -> Vec<Marking> {
        let mut markings = Vec::new();
        // Socks: feet of a colour other than the coat's, on something that stands on legs.
        if legs
            && let Some((feet, share)) = self.dominant(picture, subject, |_, v| v > 0.86)
            && feet != self.primary
            && share > 0.5
            && delta(self.swatches[feet].lab, self.swatches[self.primary].lab) > 12.0
        {
            // How far up the legs it reaches.
            let low = self
                .dominant(picture, subject, |_, v| (0.72..0.86).contains(&v))
                .is_some_and(|(i, _)| i == feet);
            markings.push(Marking {
                amount: if low { 8 } else { 4 },
                ..Marking::new(MarkingKind::Socks, unsoften(self.swatches[feet].rgb))
            });
        }
        let area = subject.pixels().count() as f32;
        // Eye patches: a dark colour of its own round each eye, far larger than an eye.
        if let Some((eu, ev)) = subject.eyes {
            let dark = (0..self.swatches.len())
                .filter(|&i| i != self.primary && self.swatches[i].lab[0] < 40.0)
                .min_by(|a, b| self.swatches[*a].lab[0].total_cmp(&self.swatches[*b].lab[0]));
            if let Some(dark) = dark {
                let mut near = 0.0;
                let mut ring = 0.0;
                for (x, y) in subject.pixels() {
                    let (u, v) = subject.place(x, y);
                    let d = ((u - eu) / 0.14).powi(2) + ((v - ev) / 0.14).powi(2);
                    if d < 1.0 {
                        ring += 1.0;
                        if self.which[picture.index(x, y)] == dark {
                            near += 1.0;
                        }
                    }
                }
                let primary_light = self.swatches[self.primary].lab[0] > 55.0;
                if ring > 0.0 && near / ring > 0.4 && primary_light {
                    markings.push(Marking {
                        amount: 6,
                        size: 5,
                        ..Marking::new(MarkingKind::EyePatches, unsoften(self.swatches[dark].rgb))
                    });
                }
            }
        }
        let Some(second) = self.secondary else {
            return markings;
        };
        let colour = unsoften(self.swatches[second].rgb);
        // The pieces the second colour makes on the body's middle.
        let mut marked = vec![false; picture.width * picture.height];
        let mut body: f32 = 0.0;
        for (x, y) in subject.pixels() {
            let (u, v) = subject.place(x, y);
            if !(0.1..0.9).contains(&u) || !(0.1..0.85).contains(&v) {
                continue;
            }
            body += 1.0;
            if self.which[picture.index(x, y)] == second {
                marked[picture.index(x, y)] = true;
            }
        }
        let pieces: Vec<Vec<usize>> = pieces(&marked, picture.width, picture.height)
            .into_iter()
            .filter(|p| p.len() >= 3)
            .collect();
        let covered = pieces.iter().map(Vec::len).sum::<usize>() as f32 / body.max(1.0);
        if pieces.is_empty() || covered < 0.03 || markings.len() >= MAX_MARKINGS {
            return markings;
        }
        let shapes: Vec<PieceShape> = pieces
            .iter()
            .map(|p| PieceShape::of(p, picture.width))
            .collect();
        let n = shapes.len() as f32;
        let long = shapes.iter().filter(|s| s.long > 2.2).count() as f32 / n;
        let upright = shapes.iter().filter(|s| s.long > 2.2 && s.upright).count() as f32 / n;
        let mean = shapes.iter().map(|s| s.area).sum::<f32>() / n / area.max(1.0);
        let holed = shapes.iter().filter(|s| s.holed).count() as f32 / n;
        let amount = ((covered * 14.0).round() as u8).clamp(2, STEPS);
        let kind = if n >= 3.0 && long > 0.5 && upright > 0.35 {
            Some(MarkingKind::Stripes)
        } else if n >= 5.0 && mean < 0.02 {
            Some(if holed > 0.3 {
                MarkingKind::Rosettes
            } else {
                MarkingKind::Spots
            })
        } else if n <= 6.0 && covered > 0.12 {
            // One great piece over the back is a saddle; a few are patches.
            let top = shapes
                .iter()
                .max_by(|a, b| a.area.total_cmp(&b.area))
                .map(|s| subject.place(s.middle.0, s.middle.1));
            match top {
                Some((u, v)) if n <= 2.0 && v < 0.45 && (0.25..0.75).contains(&u) => {
                    Some(MarkingKind::Saddle)
                }
                _ => Some(MarkingKind::Patches),
            }
        } else if covered > 0.05 {
            Some(MarkingKind::Mottle)
        } else {
            None
        };
        if let Some(kind) = kind {
            let size = ((mean.sqrt() * 40.0).round() as u8).clamp(2, 8);
            markings.push(Marking {
                amount,
                size,
                ..Marking::new(kind, colour)
            });
        }
        markings
    }
}

/// What a piece of a marking is like.
struct PieceShape {
    area: f32,
    /// How much longer it is than wide.
    long: f32,
    /// Whether it runs more up and down than along.
    upright: bool,
    /// Whether it has a hole in it, as a rosette has.
    holed: bool,
    middle: (usize, usize),
}

impl PieceShape {
    fn of(piece: &[usize], width: usize) -> Self {
        let n = piece.len() as f32;
        let (mut sx, mut sy) = (0.0, 0.0);
        for &i in piece {
            sx += (i % width) as f32;
            sy += (i / width) as f32;
        }
        let (mx, my) = (sx / n, sy / n);
        let (mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0);
        for &i in piece {
            let (dx, dy) = ((i % width) as f32 - mx, (i / width) as f32 - my);
            xx += dx * dx;
            yy += dy * dy;
            xy += dx * dy;
        }
        let (xx, yy, xy) = (xx / n, yy / n, xy / n);
        let middle_ = (xx + yy) / 2.0;
        let spread = (((xx - yy) / 2.0).powi(2) + xy * xy).sqrt();
        let (major, minor) = (middle_ + spread, (middle_ - spread).max(0.05));
        let long = (major / minor).sqrt();
        // The direction the piece runs in: within 45 degrees of upright counts as upright.
        let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
        let upright = angle.abs() > std::f32::consts::FRAC_PI_4;
        // A hole: the box has a gap in its middle that the piece goes round.
        let (l, t, r, b) = piece
            .iter()
            .fold((usize::MAX, usize::MAX, 0, 0), |(l, t, r, b), &i| {
                let (x, y) = (i % width, i / width);
                (l.min(x), t.min(y), r.max(x), b.max(y))
            });
        let centre = ((l + r) / 2, (t + b) / 2);
        let holed = (r - l) >= 3
            && (b - t) >= 3
            && !piece.contains(&(centre.1 * width + centre.0))
            && n < ((r - l + 1) * (b - t + 1)) as f32 * 0.8;
        Self {
            area: n,
            long,
            upright,
            holed,
            middle: (mx.round() as usize, my.round() as usize),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The takes.

fn takes(
    found: Vec<Found>,
    coat: &Coat,
    treatment: Treatment,
    markings: &[Marking],
    inks: &[(Slot, Ink)],
) -> Vec<Take> {
    let dress = |sculpt: &Sculpt, plain: bool| {
        let mut sculpt = sculpt.clone();
        // The plan's own treatment where the picture cannot say: feathers on a bird, thick skin
        // on an elephant read as smooth in a picture.
        let treatment = match (sculpt.plan, treatment) {
            (Plan::Percher, Treatment::Fur | Treatment::Smooth) => Treatment::Feathers,
            (Plan::LargeQuadruped, Treatment::Smooth) => Treatment::Plated,
            (_, treatment) => treatment,
        };
        sculpt.coat = Coat { treatment, ..*coat };
        for part in &mut sculpt.parts {
            if let Some((_, ink)) = inks.iter().find(|(slot, _)| *slot == part.kind.slot()) {
                part.ink = *ink;
            }
        }
        sculpt.markings = if plain { Vec::new() } else { markings.to_vec() };
        Design {
            form: Form::Sculpted {
                sculpt: sculpt.normalized(),
            },
            face: plain_face(),
        }
    };
    let percent = |likeness: f32| (likeness * 100.0).round().clamp(0.0, 100.0) as u8;
    let mut takes: Vec<Take> = Vec::new();
    let Some(best) = found.first() else {
        return takes;
    };
    let body = |sculpt: &Sculpt| sculpt.plan.label().to_owned();
    let marked = markings
        .iter()
        .map(|m| m.kind.label().to_lowercase())
        .collect::<Vec<_>>()
        .join(" and ");
    takes.push(Take {
        design: dress(&best.sculpt, false),
        likeness: percent(best.likeness),
        title: "Closest",
        summary: if marked.is_empty() {
            body(&best.sculpt)
        } else {
            format!("{}, with {marked}", body(&best.sculpt))
        },
    });
    if let Some(other) = found.iter().find(|f| f.sculpt.plan != best.sculpt.plan) {
        takes.push(Take {
            design: dress(&other.sculpt, false),
            likeness: percent(other.likeness),
            title: "Another body",
            summary: body(&other.sculpt),
        });
    }
    if let Some(animal) = found
        .iter()
        .find(|f| f.animal.is_some() && takes.iter().all(|t| t.design != dress(&f.sculpt, false)))
    {
        takes.push(Take {
            design: dress(&animal.sculpt, false),
            likeness: percent(animal.likeness),
            title: "From an animal",
            summary: format!(
                "Reshaped from the {}",
                animal.animal.unwrap_or("animal").to_lowercase()
            ),
        });
    }
    if !markings.is_empty() {
        takes.push(Take {
            design: dress(&best.sculpt, true),
            likeness: percent(best.likeness),
            title: "Plain coat",
            summary: "The closest, with no markings".into(),
        });
    }
    takes
}

// ---------------------------------------------------------------------------------------------
// Colour arithmetic.

/// A colour set so that Farm's softening (three parts colour to one of light) brings it back
/// to `rgb`, as near as softening allows.
fn unsoften(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| {
        ((i32::from(c) * 4 - 220) as f32 / 3.0)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

fn to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
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

fn to_lab(rgb: [u8; 3]) -> [f32; 3] {
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

fn from_lab(lab: [f32; 3]) -> [u8; 3] {
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
fn delta(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn chroma(lab: [f32; 3]) -> f32 {
    (lab[1] * lab[1] + lab[2] * lab[2]).sqrt()
}

fn nearest(centres: &[[f32; 3]], c: [f32; 3]) -> usize {
    (0..centres.len())
        .min_by(|a, b| delta(centres[*a], c).total_cmp(&delta(centres[*b], c)))
        .unwrap_or(0)
}

/// Up to `k` colours that `colours` gather round, with ones that look alike merged. Always the
/// same answer for the same colours: it starts from the commonest and then each farthest.
fn kmeans(colours: &[[f32; 3]], k: usize) -> Vec<[f32; 3]> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_forms::DesignRenderer;

    /// A preset drawn large, as a picture of it would be: on a plain background, finely, at
    /// rest, facing `right` or left.
    fn picture_of(id: &str, right: bool, background: [u8; 3]) -> RgbaImage {
        let preset = crate::presets::find(id).expect("the preset exists");
        let base = crate::review::stand_in();
        let canvas = DesignRenderer::intent_frame_hd(
            &preset.design,
            &base,
            formiga_forms::Intent::Idle,
            0,
            right,
            true,
            formiga_forms::DETAIL,
        );
        let mut image = RgbaImage::from_pixel(
            canvas.width(),
            canvas.height(),
            image::Rgba([background[0], background[1], background[2], 255]),
        );
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                let p = canvas.get(x as i32, y as i32);
                if p.a > 0 {
                    image.put_pixel(x, y, image::Rgba([p.r, p.g, p.b, 255]));
                }
            }
        }
        image
    }

    /// A plain drawing, as someone might make one: shapes filled in flat colours, each with a
    /// dark outline, laid one over another.
    struct Drawing(RgbaImage);

    const INK: [u8; 3] = [25, 20, 20];

    impl Drawing {
        fn new(background: [u8; 4]) -> Self {
            Self(RgbaImage::from_pixel(480, 360, image::Rgba(background)))
        }

        fn shape(&mut self, colour: [u8; 3], outline: f32, inside: impl Fn(f32, f32) -> f32) {
            // `inside` is above zero inside the shape, and about a pixel's worth per pixel.
            for y in 0..self.0.height() {
                for x in 0..self.0.width() {
                    let d = inside(x as f32 + 0.5, y as f32 + 0.5);
                    if d > outline {
                        self.0
                            .put_pixel(x, y, image::Rgba([colour[0], colour[1], colour[2], 255]));
                    } else if d > 0.0 {
                        self.0
                            .put_pixel(x, y, image::Rgba([INK[0], INK[1], INK[2], 255]));
                    }
                }
            }
        }

        fn ellipse(
            &mut self,
            colour: [u8; 3],
            outline: f32,
            (cx, cy, rx, ry): (f32, f32, f32, f32),
        ) {
            self.shape(colour, outline, |x, y| {
                let k = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
                (1.0 - k) * rx.min(ry)
            });
        }

        fn rect(&mut self, colour: [u8; 3], outline: f32, (l, t, r, b): (f32, f32, f32, f32)) {
            self.shape(colour, outline, |x, y| {
                (x - l).min(r - x).min(y - t).min(b - y)
            });
        }

        /// The drawing mirrored, to face the other way.
        fn mirrored(self) -> RgbaImage {
            image::imageops::flip_horizontal(&self.0)
        }
    }

    /// An orange cat with dark stripes and a white belly, on white, facing right.
    fn tabby() -> Drawing {
        let orange = [235, 140, 50];
        let mut d = Drawing::new([255, 255, 255, 255]);
        for x in [150.0, 195.0, 280.0, 325.0] {
            d.rect(orange, 5.0, (x, 220.0, x + 34.0, 320.0));
        }
        d.rect(orange, 5.0, (95.0, 120.0, 112.0, 200.0));
        d.ellipse(orange, 5.0, (260.0, 200.0, 130.0, 60.0));
        d.ellipse([250, 245, 235], 0.0, (260.0, 234.0, 90.0, 16.0));
        for x in [195.0, 235.0, 275.0, 315.0] {
            d.rect([120, 60, 25], 0.0, (x, 145.0, x + 14.0, 205.0));
        }
        d.ellipse(orange, 5.0, (370.0, 125.0, 70.0, 65.0));
        d.ellipse(INK, 0.0, (350.0, 115.0, 9.0, 11.0));
        d.ellipse(INK, 0.0, (395.0, 115.0, 9.0, 11.0));
        d
    }

    /// A panda on a clear background, facing right: white, with black legs, ears and eye
    /// patches.
    fn panda() -> Drawing {
        let (black, white) = ([30, 30, 35], [245, 245, 240]);
        let mut d = Drawing::new([0, 0, 0, 0]);
        for x in [110.0, 160.0, 260.0, 310.0] {
            d.rect(black, 0.0, (x, 220.0, x + 42.0, 320.0));
        }
        d.ellipse(white, 0.0, (240.0, 200.0, 140.0, 70.0));
        d.ellipse(black, 0.0, (330.0, 60.0, 25.0, 25.0));
        d.ellipse(black, 0.0, (425.0, 60.0, 25.0, 25.0));
        d.ellipse(white, 0.0, (378.0, 130.0, 75.0, 65.0));
        d.ellipse(black, 0.0, (350.0, 125.0, 17.0, 20.0));
        d.ellipse(black, 0.0, (406.0, 125.0, 17.0, 20.0));
        d.ellipse(white, 0.0, (352.0, 120.0, 5.0, 5.0));
        d.ellipse(white, 0.0, (408.0, 120.0, 5.0, 5.0));
        d
    }

    /// A blue bird with a pale breast, sitting on a branch that runs off the picture, facing
    /// right, on green.
    fn bird() -> Drawing {
        let blue = [60, 110, 210];
        let mut d = Drawing::new([150, 200, 120, 255]);
        d.rect([110, 75, 40], 0.0, (0.0, 292.0, 480.0, 308.0));
        d.rect([240, 150, 30], 0.0, (212.0, 280.0, 218.0, 300.0));
        d.rect([240, 150, 30], 0.0, (258.0, 280.0, 264.0, 300.0));
        d.ellipse([40, 70, 160], 5.0, (130.0, 250.0, 70.0, 22.0));
        d.ellipse(blue, 5.0, (230.0, 215.0, 90.0, 75.0));
        d.ellipse([240, 220, 170], 0.0, (240.0, 250.0, 60.0, 35.0));
        d.ellipse(blue, 5.0, (310.0, 130.0, 50.0, 50.0));
        d.rect([240, 150, 30], 4.0, (355.0, 120.0, 395.0, 140.0));
        d.ellipse(INK, 0.0, (326.0, 118.0, 8.0, 8.0));
        d
    }

    /// A long, low green lizard with yellow spots on sand, facing right.
    fn lizard() -> Drawing {
        let green = [80, 160, 70];
        let mut d = Drawing::new([225, 205, 160, 255]);
        d.ellipse(green, 4.0, (110.0, 245.0, 90.0, 12.0));
        for x in [175.0, 215.0, 300.0, 340.0] {
            d.rect(green, 3.0, (x, 250.0, x + 14.0, 292.0));
        }
        d.ellipse(green, 4.0, (265.0, 242.0, 125.0, 32.0));
        d.ellipse(green, 4.0, (405.0, 235.0, 45.0, 25.0));
        for x in [180.0, 215.0, 250.0, 285.0, 320.0, 355.0] {
            d.ellipse([230, 220, 60], 0.0, (x, 238.0, 8.0, 8.0));
        }
        d.ellipse(INK, 0.0, (425.0, 228.0, 6.0, 6.0));
        d
    }

    fn sculpt_of(take: &Take) -> &Sculpt {
        take.design.form.sculpt().expect("every take is sculpted")
    }

    #[test]
    fn colours_survive_the_round_trip_through_lab() {
        for rgb in [[0, 0, 0], [255, 255, 255], [200, 120, 40], [30, 90, 200]] {
            let back = from_lab(to_lab(rgb));
            for (a, b) in rgb.iter().zip(back) {
                assert!(a.abs_diff(b) <= 1, "{rgb:?} came back as {back:?}");
            }
        }
    }

    #[test]
    fn a_colour_unsoftened_shows_as_itself_once_farm_softens_it() {
        for c in [120_u8, 180, 230] {
            let soft = (u16::from(unsoften([c; 3])[0]) * 3 + 220) / 4;
            assert!(soft.abs_diff(u16::from(c)) <= 1);
        }
    }

    #[test]
    fn a_picture_of_a_whale_reads_as_a_floater() {
        let base = crate::review::stand_in();
        let takes = read_image(
            &picture_of("animal.humpback_whale", true, [250, 250, 250]),
            &base,
        )
        .expect("it reads");
        assert_eq!(sculpt_of(&takes[0]).plan, Plan::Floater);
        assert!(takes[0].likeness >= 70, "{}", takes[0].likeness);
    }

    #[test]
    fn a_picture_facing_left_is_read_facing_right() {
        let base = crate::review::stand_in();
        let right = read_image(&picture_of("animal.giraffe", true, [240, 248, 255]), &base)
            .expect("it reads");
        let left = read_image(&picture_of("animal.giraffe", false, [240, 248, 255]), &base)
            .expect("it reads");
        assert_eq!(sculpt_of(&left[0]).plan, sculpt_of(&right[0]).plan);
        assert!(left[0].likeness.abs_diff(right[0].likeness) <= 5);
    }

    /// A photo taken with the phone on its side is stored as it was taken, with a note to turn
    /// it; it is read turned.
    #[test]
    fn a_picture_is_read_the_right_way_up() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 40, 20);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("a header");
            // "Turn a quarter clockwise to view", in the TIFF form an eXIf chunk holds.
            let exif = [
                b'M', b'M', 0, 42, 0, 0, 0, 8, 0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0,
                0, 0, 0, 0, 0,
            ];
            writer
                .write_chunk(png::chunk::ChunkType(*b"eXIf"), &exif)
                .expect("the note");
            writer
                .write_image_data(&[200; 40 * 20 * 4])
                .expect("the pixels");
        }
        let picture = decode(&bytes).expect("it opens");
        assert_eq!((picture.width(), picture.height()), (20, 40));
    }

    #[test]
    fn a_picture_with_nothing_in_it_says_so() {
        let base = crate::review::stand_in();
        let clear = RgbaImage::from_pixel(40, 40, image::Rgba([0, 0, 0, 0]));
        assert!(read_image(&clear, &base).is_err());
        assert!(decode(b"not a picture").is_err());
    }

    /// Read a drawing, thumbnailed as a picture opened from a file is.
    fn read(image: &RgbaImage) -> Vec<Take> {
        let small = image::DynamicImage::ImageRgba8(image.clone())
            .thumbnail(ANALYSIS, ANALYSIS)
            .to_rgba8();
        read_image(&small, &crate::review::stand_in()).expect("the drawing reads")
    }

    #[test]
    fn a_tabby_reads_as_a_striped_orange_four_legged_creature() {
        let takes = read(&tabby().0);
        let sculpt = sculpt_of(&takes[0]);
        assert!(
            matches!(
                sculpt.plan,
                Plan::CompactQuadruped | Plan::LargeQuadruped | Plan::TallQuadruped
            ),
            "{:?}",
            sculpt.plan
        );
        assert!(
            sculpt
                .markings
                .iter()
                .any(|m| m.kind == MarkingKind::Stripes),
            "{:?}",
            sculpt.markings
        );
        // Orange: more red than blue, by a long way, and the belly paler than the coat.
        let [r, _, b] = sculpt.coat.primary;
        assert!(r > b + 80, "{:?}", sculpt.coat.primary);
        let lightness = |c: [u8; 3]| to_lab(c)[0];
        assert!(lightness(sculpt.coat.underside) > lightness(sculpt.coat.primary));
    }

    #[test]
    fn a_tabby_facing_left_reads_as_one_facing_right() {
        let right = read(&tabby().0);
        let left = read(&tabby().mirrored());
        assert_eq!(sculpt_of(&left[0]).plan, sculpt_of(&right[0]).plan);
        assert!(left[0].likeness.abs_diff(right[0].likeness) <= 3);
    }

    #[test]
    fn a_pandas_black_comes_through_on_its_legs_and_round_its_eyes() {
        let takes = read(&panda().0);
        let sculpt = sculpt_of(&takes[0]);
        let kinds: Vec<MarkingKind> = sculpt.markings.iter().map(|m| m.kind).collect();
        assert!(kinds.contains(&MarkingKind::Socks), "{kinds:?}");
        assert!(kinds.contains(&MarkingKind::EyePatches), "{kinds:?}");
        assert!(
            to_lab(sculpt.coat.primary)[0] > 80.0,
            "{:?}",
            sculpt.coat.primary
        );
        let feet = sculpt.part(Slot::Feet).expect("a panda stands on feet");
        assert!(to_lab(sculpt.coat.ink(feet.ink))[0] < 30.0);
    }

    #[test]
    fn a_bird_on_a_branch_reads_as_a_percher() {
        let takes = read(&bird().0);
        assert_eq!(sculpt_of(&takes[0]).plan, Plan::Percher);
        // Its beak is the orange of the picture's.
        let sculpt = sculpt_of(&takes[0]);
        let beak = sculpt.part(Slot::Snout).expect("a percher has a beak");
        let [r, g, b] = sculpt.coat.ink(beak.ink);
        assert!(r > g && g > b && r > 150, "{:?}", [r, g, b]);
    }

    #[test]
    fn a_lizard_reads_as_a_crawler_without_a_shell() {
        let takes = read(&lizard().0);
        let sculpt = sculpt_of(&takes[0]);
        assert_eq!(sculpt.plan, Plan::Crawler);
        assert!(sculpt.part(Slot::Shell).is_none());
        let [r, g, b] = sculpt.coat.primary;
        assert!(g > r && g > b, "{:?}", sculpt.coat.primary);
    }

    #[test]
    fn every_take_is_a_valid_design_and_they_differ() {
        let base = crate::review::stand_in();
        for id in [
            "animal.bald_eagle",
            "animal.sea_turtle",
            "animal.african_elephant",
        ] {
            let takes = read_image(&picture_of(id, true, [255, 255, 255]), &base).expect("reads");
            assert!(takes.len() >= 2, "{id}");
            for take in &takes {
                take.design.validate().expect("a take is a valid design");
            }
            for (i, a) in takes.iter().enumerate() {
                for b in takes.iter().skip(i + 1) {
                    assert_ne!(a.design, b.design, "{id}");
                }
            }
        }
    }
}
