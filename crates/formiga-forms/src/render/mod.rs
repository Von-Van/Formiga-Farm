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
                let (sheet, figure) = sculpted(sculpt, &genome, clip, frame, reduce_motion, 1);
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

    /// [`Self::body_frame`] in high definition, `detail` pixels to each frame pixel (see
    /// [`Self::frame_hd`]): what Desktop bakes into an atlas to draw a reshaped companion finely.
    /// The face anchor stays in the frame's own 48 pixels, as every anchor does; the face goes
    /// on it from [`Self::face_frame_hd`], its middle on the anchor.
    pub fn body_frame_hd(
        design: &Design,
        base: &AppearanceGenome,
        clip: impl Into<BodyClip>,
        frame: u8,
        reduce_motion: bool,
        detail: u32,
    ) -> RenderedBodyFrame {
        let clip = clip.into();
        let res = detail_of(detail);
        match &design.form {
            Form::Sculpted { sculpt } => {
                let genome = design.genome(base);
                let (sheet, figure) = sculpted(sculpt, &genome, clip, frame, reduce_motion, res);
                let canvas = sheet.canvas;
                RenderedBodyFrame {
                    face_anchor: point(figure.face),
                    alpha_mask: AlphaMask::from_canvas(&canvas),
                    canvas,
                }
            }
            _ => {
                let body = CreatureRenderer::render_body_frame(
                    &design.genome(base),
                    clip,
                    frame,
                    reduce_motion,
                );
                let canvas = blocks(&body.canvas, res);
                RenderedBodyFrame {
                    face_anchor: body.face_anchor,
                    alpha_mask: AlphaMask::from_canvas(&canvas),
                    canvas,
                }
            }
        }
    }

    /// The face a design wears in high definition, facing right, `detail` times the size of
    /// [`formiga_art::FACE_FRAME_SIZE`]: Formiga's own face for `face_state`, in a sculpted
    /// form's eye colour and with its corners rounded, or a companion's face drawn larger.
    pub fn face_frame_hd(
        design: &Design,
        base: &AppearanceGenome,
        face_state: FaceRenderState,
        detail: u32,
    ) -> Canvas {
        let res = detail_of(detail);
        let genome = design.genome(base);
        match &design.form {
            Form::Sculpted { sculpt } => sculpted_face(sculpt, &genome, face_state, res),
            _ => blocks(
                &CreatureRenderer::render_face_frame(&genome, face_state),
                res,
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
        compose(
            design,
            base,
            clip.into(),
            frame,
            facing_right,
            reduce_motion,
            face_state,
            1,
        )
    }

    /// The same frame in high definition, `detail` pixels across for each of its pixels, from 1
    /// to [`crate::MAX_DETAIL`]; 1 is exactly [`Self::frame`]. Everything stands where it stands in
    /// [`Self::frame`]: a sculpted form is painted in the same style, pixel art with a
    /// companion's outline, with finer curves, three tones of shade and its coat's grain, and
    /// wears Formiga's own face with its corners rounded. A creature that keeps its body is
    /// Desktop's own drawing, each pixel drawn as a square.
    #[allow(clippy::too_many_arguments)]
    pub fn frame_hd(
        design: &Design,
        base: &AppearanceGenome,
        clip: impl Into<BodyClip>,
        frame: u8,
        facing_right: bool,
        reduce_motion: bool,
        face_state: FaceRenderState,
        detail: u32,
    ) -> Canvas {
        compose(
            design,
            base,
            clip.into(),
            frame,
            facing_right,
            reduce_motion,
            face_state,
            detail_of(detail),
        )
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
        Self::frame(
            design,
            base,
            intent.clip(),
            frame,
            facing_right,
            reduce_motion,
            intent_face(intent),
        )
    }

    /// [`Self::intent_frame`] in high definition (see [`Self::frame_hd`]).
    pub fn intent_frame_hd(
        design: &Design,
        base: &AppearanceGenome,
        intent: Intent,
        frame: u8,
        facing_right: bool,
        reduce_motion: bool,
        detail: u32,
    ) -> Canvas {
        Self::frame_hd(
            design,
            base,
            intent.clip(),
            frame,
            facing_right,
            reduce_motion,
            intent_face(intent),
            detail,
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
                let (sheet, _) = sculpted(sculpt, &genome, ActionKind::Idle.into(), 0, true, 1);
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
}

/// The detail a frame is drawn at: from 1 (exactly the 48-pixel frame) to
/// [`crate::MAX_DETAIL`] pixels to a frame pixel.
fn detail_of(detail: u32) -> i32 {
    detail.clamp(1, crate::MAX_DETAIL) as i32
}

/// The face a preview of `intent` wears.
fn intent_face(intent: Intent) -> FaceRenderState {
    let BodyClip::Action(action) = intent.clip() else {
        unreachable!("every intent shows an action")
    };
    FaceRenderState {
        expression: expression_for(intent),
        eyelids: if action == ActionKind::Sleep {
            formiga_art::EyelidPose::Closed
        } else {
            formiga_art::EyelidPose::Open
        },
        gaze: formiga_art::GazeDirection::new(0, 0),
    }
}

/// A whole frame, `res` pixels to a frame pixel.
#[allow(clippy::too_many_arguments)]
fn compose(
    design: &Design,
    base: &AppearanceGenome,
    clip: BodyClip,
    frame: u8,
    facing_right: bool,
    reduce_motion: bool,
    face_state: FaceRenderState,
    res: i32,
) -> Canvas {
    let genome = design.genome(base);
    let Form::Sculpted { sculpt } = &design.form else {
        let canvas = CreatureRenderer::render_composited_frame(
            &genome,
            clip,
            frame,
            facing_right,
            reduce_motion,
            face_state,
        );
        return if res == 1 {
            canvas
        } else {
            blocks(&canvas, res)
        };
    };
    let (sheet, figure) = sculpted(sculpt, &genome, clip, frame, reduce_motion, res);
    let mut canvas = sheet.canvas;
    let mut anchor = point(figure.face);
    let mut face_state = face_state;
    if !facing_right {
        canvas.mirror_horizontal();
        anchor.x = FRAME_SIZE as i32 - anchor.x;
        face_state.gaze.x = -face_state.gaze.x;
    }
    let mut face = sculpted_face(sculpt, &genome, face_state, res);
    if !facing_right {
        face.mirror_horizontal();
    }
    let origin_x = (anchor.x - FACE_FRAME_SIZE as i32 / 2) * res;
    let origin_y = (anchor.y - FACE_FRAME_SIZE as i32 / 2) * res;
    for y in 0..face.height() as i32 {
        for x in 0..face.width() as i32 {
            let pixel = face.get(x, y);
            if pixel.a > 0 {
                canvas.set(origin_x + x, origin_y + y, pixel);
            }
        }
    }
    canvas
}

/// Formiga's own face for `face_state` in a sculpted form's eye colour, `res` pixels to each of
/// its own, facing right.
fn sculpted_face(
    sculpt: &Sculpt,
    genome: &AppearanceGenome,
    face_state: FaceRenderState,
    res: i32,
) -> Canvas {
    let mut face = CreatureRenderer::render_face_frame(genome, face_state);
    recolor_eyes(&mut face, genome, sculpt.coat.eyes);
    if res == 1 {
        face
    } else {
        rounded_face(&face, res)
    }
}

/// `canvas` with each pixel drawn as a `res` by `res` square.
fn blocks(canvas: &Canvas, res: i32) -> Canvas {
    let mut out = Canvas::new(canvas.width() * res as u32, canvas.height() * res as u32);
    for y in 0..out.height() as i32 {
        for x in 0..out.width() as i32 {
            out.set(x, y, canvas.get(x / res, y / res));
        }
    }
    out
}

/// A pixel face drawn `res` times larger with its corners rounded, as a pixel artist would
/// redraw it larger: each pixel doubled, tripled or doubled twice, so that a corner between two
/// runs of one colour is filled and a lone corner cut. Every expression keeps every feature,
/// still in flat pixels, only rounder.
fn rounded_face(face: &Canvas, res: i32) -> Canvas {
    match res {
        2 => scale2x(face),
        3 => scale3x(face),
        4 => scale2x(&scale2x(face)),
        _ => face.clone(),
    }
}

/// One pass of the Scale3x pixel-art enlargement.
fn scale3x(canvas: &Canvas) -> Canvas {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let mut out = Canvas::new(w as u32 * 3, h as u32 * 3);
    let at = |x: i32, y: i32| canvas.get(x.clamp(0, w - 1), y.clamp(0, h - 1));
    for y in 0..h {
        for x in 0..w {
            let (a, b, c) = (at(x - 1, y - 1), at(x, y - 1), at(x + 1, y - 1));
            let (d, e, f) = (at(x - 1, y), at(x, y), at(x + 1, y));
            let (g, hh, i) = (at(x - 1, y + 1), at(x, y + 1), at(x + 1, y + 1));
            let mut cells = [e; 9];
            if b != hh && d != f {
                if d == b {
                    cells[0] = d;
                }
                if (d == b && e != c) || (b == f && e != a) {
                    cells[1] = b;
                }
                if b == f {
                    cells[2] = f;
                }
                if (d == b && e != g) || (d == hh && e != a) {
                    cells[3] = d;
                }
                if (b == f && e != i) || (hh == f && e != c) {
                    cells[5] = f;
                }
                if d == hh {
                    cells[6] = d;
                }
                if (d == hh && e != i) || (hh == f && e != g) {
                    cells[7] = hh;
                }
                if hh == f {
                    cells[8] = f;
                }
            }
            for (n, cell) in cells.into_iter().enumerate() {
                out.set(x * 3 + n as i32 % 3, y * 3 + n as i32 / 3, cell);
            }
        }
    }
    out
}

/// One pass of the Scale2x pixel-art enlargement.
fn scale2x(canvas: &Canvas) -> Canvas {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    let mut out = Canvas::new(w as u32 * 2, h as u32 * 2);
    let at = |x: i32, y: i32| canvas.get(x.clamp(0, w - 1), y.clamp(0, h - 1));
    for y in 0..h {
        for x in 0..w {
            let e = at(x, y);
            let (b, d, f, hh) = (at(x, y - 1), at(x - 1, y), at(x + 1, y), at(x, y + 1));
            let (mut e0, mut e1, mut e2, mut e3) = (e, e, e, e);
            if b != hh && d != f {
                if d == b {
                    e0 = d;
                }
                if b == f {
                    e1 = f;
                }
                if d == hh {
                    e2 = d;
                }
                if hh == f {
                    e3 = f;
                }
            }
            out.set(x * 2, y * 2, e0);
            out.set(x * 2 + 1, y * 2, e1);
            out.set(x * 2, y * 2 + 1, e2);
            out.set(x * 2 + 1, y * 2 + 1, e3);
        }
    }
    out
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
    res: i32,
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
    let laid_out = figure.clone();
    let mut figure = figure;
    let (mut moved_x, mut moved_y) = (0, 0);
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
            (moved_x, moved_y) = (dx, dy);
        }
    }
    if res > 1 {
        // The same figure painted finely, held exactly where the frame above holds it.
        let mut fine = Sheet::with_res(res);
        draw::draw(&mut fine, &sculpt, &laid_out, pose, eye_spacing);
        if moved_x != 0 || moved_y != 0 {
            fine.canvas.translate(moved_x * res, moved_y * res);
        }
        return (fine, figure);
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
