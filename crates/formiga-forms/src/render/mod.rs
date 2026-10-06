//! Drawing a design, one 48-pixel frame at a time, the same size and the same way as every other
//! Formiga companion. A companion recipe, or a look from before recipes, is drawn by Desktop's own
//! renderer, untouched. A sculpted form is painted here, then given the very face Desktop's
//! renderer draws for a companion, so it keeps every expression.

mod draw;
mod figure;
mod motion;
mod paint;

pub use motion::Intent;

use crate::{Design, Form, Sculpt};
use figure::{FLOOR, Figure, HOVER};
use formiga_art::{
    AlphaMask, BodyClip, Canvas, CreatureRenderer, FACE_FRAME_SIZE, FRAME_SIZE, FaceRenderState,
    PixelPoint, RenderedBodyFrame, Rgba,
};
use formiga_core::{ActionKind, AppearanceGenome};
use paint::Sheet;

/// The largest a sculpted figure may stand across the frame, and how far above the ground its
/// top may reach, leaving room for a hop.
const MAX_WIDTH: u32 = 44;
const HEADROOM: u32 = 7;

/// Where a design's figure stands in its frame, in frame pixels with the figure facing right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchors {
    /// The row its feet rest on when standing, or that a floater hovers above.
    pub ground_row: i32,
    /// The middle of its face.
    pub face: PixelPoint,
    /// Where someone picks it up.
    pub scruff: PixelPoint,
    /// The pixels it covers at rest: left, top, right, bottom, inclusive.
    pub bounds: (u32, u32, u32, u32),
}

/// How a design is drawn.
pub struct DesignRenderer;

impl DesignRenderer {
    /// One body frame of `clip`, facing right, with no face: what Desktop bakes into an atlas.
    /// `base` is the creature's genome, which a design is drawn over (see [`Design::genome`]).
    pub fn body_frame(
        design: &Design,
        base: &AppearanceGenome,
        clip: impl Into<BodyClip>,
        frame: u8,
        reduce_motion: bool,
    ) -> RenderedBodyFrame {
        let clip = clip.into();
        match &design.form {
            Form::Sculpted { sculpt } => {
                let genome = design.genome(base);
                let (sheet, figure) = sculpted(sculpt, &genome, clip, frame, reduce_motion);
                let canvas = sheet.canvas;
                RenderedBodyFrame {
                    face_anchor: point(figure.face),
                    alpha_mask: AlphaMask::from_canvas(&canvas),
                    canvas,
                }
            }
            _ => CreatureRenderer::render_body_frame(
                &design.genome(base),
                clip,
                frame,
                reduce_motion,
            ),
        }
    }

    /// One whole frame, face and all, as it would be drawn on the desktop.
    pub fn frame(
        design: &Design,
        base: &AppearanceGenome,
        clip: impl Into<BodyClip>,
        frame: u8,
        facing_right: bool,
        reduce_motion: bool,
        face_state: FaceRenderState,
    ) -> Canvas {
        let clip = clip.into();
        let genome = design.genome(base);
        let Form::Sculpted { sculpt } = &design.form else {
            return CreatureRenderer::render_composited_frame(
                &genome,
                clip,
                frame,
                facing_right,
                reduce_motion,
                face_state,
            );
        };
        let mut body = Self::body_frame(design, base, clip, frame, reduce_motion);
        let mut face_state = face_state;
        if !facing_right {
            body.canvas.mirror_horizontal();
            body.face_anchor.x = FRAME_SIZE as i32 - body.face_anchor.x;
            face_state.gaze.x = -face_state.gaze.x;
        }
        let mut face = CreatureRenderer::render_face_frame(&genome, face_state);
        recolor_eyes(&mut face, &genome, sculpt.coat.eyes);
        if !facing_right {
            face.mirror_horizontal();
        }
        let origin_x = body.face_anchor.x - FACE_FRAME_SIZE as i32 / 2;
        let origin_y = body.face_anchor.y - FACE_FRAME_SIZE as i32 / 2;
        for y in 0..FACE_FRAME_SIZE as i32 {
            for x in 0..FACE_FRAME_SIZE as i32 {
                let pixel = face.get(x, y);
                if pixel.a > 0 {
                    body.canvas.set(origin_x + x, origin_y + y, pixel);
                }
            }
        }
        body.canvas
    }

    /// A whole frame of the clip that shows `intent`, with the face Desktop shows for it.
    pub fn intent_frame(
        design: &Design,
        base: &AppearanceGenome,
        intent: Intent,
        frame: u8,
        facing_right: bool,
        reduce_motion: bool,
    ) -> Canvas {
        let clip = intent.clip();
        let BodyClip::Action(action) = clip else {
            unreachable!("every intent shows an action")
        };
        let mut state = formiga_art::FaceRenderState {
            expression: expression_for(intent),
            eyelids: formiga_art::EyelidPose::Open,
            gaze: formiga_art::GazeDirection::new(0, 0),
        };
        if action == ActionKind::Sleep {
            state.eyelids = formiga_art::EyelidPose::Closed;
        }
        Self::frame(
            design,
            base,
            clip,
            frame,
            facing_right,
            reduce_motion,
            state,
        )
    }

    /// How many frames, and how many a second, `intent`'s clip plays on the desktop.
    pub fn timing(intent: Intent) -> (u8, u8) {
        let spec = formiga_art::AnimationSpec::for_clip(intent.clip());
        (spec.frames, spec.fps)
    }

    /// Empty rows beneath the figure when it rests, as Desktop's own renderer measures them, so
    /// it can be stood on whatever it rests on. A floater's are measured to the ground it hovers
    /// above, never to its belly, so it is never set down on the ground.
    pub fn resting_baseline(design: &Design, base: &AppearanceGenome, reduce_motion: bool) -> u32 {
        match &design.form {
            Form::Sculpted { sculpt } => {
                // Measured from its own resting frame, as Desktop measures a companion's; a
                // floater's is measured to the ground below it, so it is never set down there.
                let genome = design.genome(base);
                let (sheet, _) = sculpted(sculpt, &genome, ActionKind::Idle.into(), 0, true);
                let bottom = sheet
                    .canvas
                    .alpha_bounds()
                    .map_or(FLOOR as u32, |(_, _, _, bottom)| bottom);
                let hover = if sculpt.plan == crate::Plan::Floater {
                    HOVER as u32
                } else {
                    0
                };
                (FRAME_SIZE - 1 - bottom).saturating_sub(hover)
            }
            _ => CreatureRenderer::resting_baseline(&design.genome(base), reduce_motion),
        }
    }

    /// Where the figure stands in its frame, at rest.
    pub fn anchors(design: &Design, base: &AppearanceGenome) -> Anchors {
        let body = Self::body_frame(design, base, ActionKind::Idle, 0, true);
        let bounds = body.canvas.alpha_bounds().unwrap_or((0, 0, 0, 0));
        let baseline = Self::resting_baseline(design, base, true) as i32;
        let scruff = match &design.form {
            Form::Sculpted { sculpt } => {
                let genome = design.genome(base);
                let (_, figure) = sculpted(sculpt, &genome, ActionKind::Idle.into(), 0, true);
                point(figure.scruff)
            }
            _ => PixelPoint {
                x: body.face_anchor.x - 4,
                y: bounds.1 as i32 + 2,
            },
        };
        Anchors {
            ground_row: FRAME_SIZE as i32 - 1 - baseline,
            face: body.face_anchor,
            scruff,
            bounds,
        }
    }
}

fn point((x, y): (f32, f32)) -> PixelPoint {
    PixelPoint {
        x: x.round() as i32,
        y: y.round() as i32,
    }
}

/// What face a preview of `intent` wears.
fn expression_for(intent: Intent) -> formiga_art::ExpressionKind {
    use formiga_art::ExpressionKind as E;
    match intent {
        Intent::Idle => E::Content,
        Intent::Move => E::Content,
        Intent::React => E::Startled,
        Intent::Inspect => E::Curious,
        Intent::Rest => E::Sleepy,
        Intent::Celebrate => E::Joy,
        Intent::Social => E::Affectionate,
        Intent::Held => E::Pleading,
    }
}

/// How large a sculpted form is drawn: its creature's size, as Desktop draws a companion's.
fn size_of(genome: &AppearanceGenome) -> f32 {
    (f32::from(genome.logical_size) / 38.0).clamp(0.55, 1.2)
}

/// Paint a sculpted form, fitted to its frame: a form too large for the frame is drawn smaller
/// as a whole, never cropped, and the figure is set in the middle. The fit is measured once at
/// rest, so every frame of every clip shares it and nothing jumps between frames.
fn sculpted(
    sculpt: &Sculpt,
    genome: &AppearanceGenome,
    clip: BodyClip,
    frame: u8,
    reduce_motion: bool,
) -> (Sheet, Figure) {
    let sculpt = sculpt.normalized();
    let eye_spacing = genome.face.eye_spacing;
    let (scale, dx) = fit(&sculpt, size_of(genome), eye_spacing);
    let pose = motion::pose(sculpt.plan, clip, frame, reduce_motion);
    let figure = Figure::lay_out(&sculpt, scale, pose).shifted(dx);
    let mut sheet = Sheet::new();
    draw::draw(&mut sheet, &sculpt, &figure, pose, eye_spacing);
    // A leap that would reach past the frame is held inside it, a pixel from the edge, as
    // Desktop's atlas keeps every companion's.
    let mut figure = figure;
    if let Some((left, top, right, bottom)) = sheet.canvas.alpha_bounds() {
        let edge = FRAME_SIZE as i32 - 2;
        let shift = |low: u32, high: u32| {
            if (low as i32) < 1 {
                1 - low as i32
            } else if high as i32 > edge {
                edge - high as i32
            } else {
                0
            }
        };
        let (dx, dy) = (shift(left, right), shift(top, bottom));
        if dx != 0 || dy != 0 {
            sheet.canvas.translate(dx, dy);
            figure = figure.shifted(dx as f32);
            figure.face.1 += dy as f32;
            figure.scruff.1 += dy as f32;
        }
    }
    (sheet, figure)
}

/// The fit of `sculpt`, remembered: every frame of every clip shares it, and measuring it means
/// drawing the form a few times over.
fn fit(sculpt: &Sculpt, size: f32, eye_spacing: u8) -> (f32, f32) {
    type Key = (Sculpt, u32, u8);
    thread_local! {
        static FITS: std::cell::RefCell<std::collections::HashMap<Key, (f32, f32)>> =
            std::cell::RefCell::default();
    }
    let key = (sculpt.clone(), size.to_bits(), eye_spacing);
    if let Some(fit) = FITS.with(|fits| fits.borrow().get(&key).copied()) {
        return fit;
    }
    let fit = measure_fit(sculpt, size, eye_spacing);
    FITS.with(|fits| {
        let mut fits = fits.borrow_mut();
        if fits.len() >= 256 {
            fits.clear();
        }
        fits.insert(key, fit);
    });
    fit
}

fn measure_fit(sculpt: &Sculpt, size: f32, eye_spacing: u8) -> (f32, f32) {
    let rest = motion::pose(sculpt.plan, ActionKind::Idle.into(), 0, true);
    let mut scale = size;
    for _ in 0..8 {
        let figure = Figure::lay_out(sculpt, scale, rest);
        let mut sheet = Sheet::new();
        draw::draw(&mut sheet, sculpt, &figure, rest, eye_spacing);
        let Some((left, top, right, _)) = sheet.canvas.alpha_bounds() else {
            return (scale, 0.0);
        };
        let width = right - left + 1;
        let wide = width as f32 / MAX_WIDTH as f32;
        let tall = (FLOOR - HEADROOM as f32 + 1.0) / (FLOOR - top as f32 + 1.0);
        let over = wide.max(1.0 / tall);
        if over <= 1.0 {
            let middle = (left + right) as f32 / 2.0;
            return (scale, (FRAME_SIZE as f32 / 2.0 - 0.5 - middle).round());
        }
        scale *= (1.0 / over).max(0.7) * 0.98;
    }
    (scale, 0.0)
}

/// The eyes in a sculpted form's own colour: every pixel of the face drawn in the palette's eye
/// ink is redrawn in `eyes`.
fn recolor_eyes(face: &mut Canvas, genome: &AppearanceGenome, eyes: [u8; 3]) {
    let ink = formiga_art::palette_for(genome).eye;
    let to = Rgba::new(eyes[0], eyes[1], eyes[2], 255);
    for y in 0..face.height() as i32 {
        for x in 0..face.width() as i32 {
            let pixel = face.get(x, y);
            if pixel.a > 0 && (pixel.r, pixel.g, pixel.b) == (ink.r, ink.g, ink.b) {
                face.set(x, y, Rgba { a: pixel.a, ..to });
            }
        }
    }
}
