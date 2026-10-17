//! A design drawn finely for Farm's own window and review sheets: Desktop's renderer paints the
//! body and the face in detail ([`CreatureRenderer::render_dressed_body_frame_in_detail`],
//! [`CreatureRenderer::render_face_frame_in_detail`]), and this puts the one on the other as
//! [`CreatureRenderer::render_composited_frame`] does at frame size. Nothing here draws a pixel
//! of its own: at one pixel to the unit it is exactly
//! [`formiga_art::forms::DesignRenderer::intent_frame`].

use formiga_art::forms::{FINEST_DETAIL, Intent};
use formiga_art::{
    BodyClip, Canvas, CreatureRenderer, ExpressionKind, EyelidPose, FACE_FRAME_SIZE, FRAME_SIZE,
    FaceRenderState, GazeDirection,
};
use formiga_core::forms::Design;
use formiga_core::{ActionKind, AppearanceGenome};

/// How many pixels across each of a frame's 48 Farm draws a sculpted form with on its stage and
/// in its review sheets: the same frame, finer pixel art in the same style.
pub const DETAIL: u32 = 2;

/// The most pixels across each frame pixel a frame can be drawn with: as finely as Desktop
/// draws a sculpted form on the largest screens.
pub const MAX_DETAIL: u32 = FINEST_DETAIL;

/// `design` as Formiga Hill and Home will draw it: the creature's look carried on a trip, exactly
/// as Desktop sends it. A trip leaves a sculpted form behind, so it travels as the companion
/// recipe Desktop keeps beside it ([`Design::fallback_recipe`]), in its colours. Anything a trip
/// could not carry is drawn as Farm has it.
pub fn as_it_travels(design: &Design, base: &AppearanceGenome) -> Design {
    let genome = design.genome(base);
    formiga_travel::TravelAppearance::from(&genome)
        .to_genome()
        .map_or_else(|_| design.clone(), |travelled| Design::of(&travelled))
}

/// The face Desktop shows for `intent`, as `DesignRenderer::intent_frame` draws it.
fn intent_face(intent: Intent) -> FaceRenderState {
    let BodyClip::Action(action) = intent.clip() else {
        unreachable!("every intent shows an action")
    };
    use ExpressionKind as E;
    let expression = match intent {
        Intent::Idle | Intent::Move => E::Content,
        Intent::React => E::Startled,
        Intent::Inspect => E::Curious,
        Intent::Rest => E::Sleepy,
        Intent::Celebrate => E::Joy,
        Intent::Social => E::Affectionate,
        Intent::Held => E::Pleading,
    };
    FaceRenderState {
        expression,
        eyelids: if action == ActionKind::Sleep {
            EyelidPose::Closed
        } else {
            EyelidPose::Open
        },
        gaze: GazeDirection::new(0, 0),
    }
}

/// A whole frame of the clip that shows `intent`, face and all, `detail` pixels across for each
/// of its own, from 1 to [`MAX_DETAIL`]. A sculpted form is painted that finely and wears its face
/// redrawn larger; a creature that keeps its body is Desktop's own drawing, each pixel a square.
pub fn intent_frame_hd(
    design: &Design,
    base: &AppearanceGenome,
    intent: Intent,
    frame: u8,
    facing_right: bool,
    reduce_motion: bool,
    detail: u32,
) -> Canvas {
    let detail = detail.clamp(1, MAX_DETAIL);
    let genome = design.genome(base);
    let (body, mut canvas) = CreatureRenderer::render_dressed_body_frame_in_detail(
        &genome,
        None,
        intent.clip(),
        frame,
        reduce_motion,
        detail,
    );
    let mut anchor = body.face_anchor;
    let mut state = intent_face(intent);
    if !facing_right {
        canvas.mirror_horizontal();
        // A 16-pixel face is centred between columns, so its anchor mirrors as Desktop's does.
        anchor.x = FRAME_SIZE as i32 - anchor.x;
        state.gaze.x = -state.gaze.x;
    }
    let mut face = CreatureRenderer::render_face_frame_in_detail(&genome, state, detail);
    if !facing_right {
        face.mirror_horizontal();
    }
    let res = detail as i32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_art::forms::DesignRenderer;
    use formiga_core::forms::{Form, Plan, Sculpt, plain_face};

    fn designs() -> Vec<Design> {
        let mut designs: Vec<Design> = Plan::ALL
            .iter()
            .map(|&plan| Design {
                form: Form::Sculpted {
                    sculpt: Sculpt::starter(plan),
                },
                face: plain_face(),
            })
            .collect();
        designs.push(
            crate::presets::find("animal.giant_panda")
                .unwrap()
                .design
                .clone(),
        );
        designs.push(Design::of(&crate::review::stand_in()));
        designs
    }

    #[test]
    fn a_sculpted_form_travels_as_its_nearest_companion_body_and_a_companion_as_itself() {
        let base = crate::review::stand_in();
        for design in designs() {
            let travelled = as_it_travels(&design, &base);
            assert!(travelled.form.sculpt().is_none());
            match &design.form {
                Form::Sculpted { .. } => assert_eq!(
                    travelled.form,
                    Form::Companion {
                        recipe: design.fallback_recipe(base.design).unwrap()
                    }
                ),
                _ => assert_eq!(travelled.genome(&base), design.genome(&base)),
            }
            assert_eq!(travelled.face, design.face);
        }
    }

    #[test]
    fn at_one_pixel_to_the_unit_a_frame_is_exactly_the_one_desktop_draws() {
        let base = crate::review::stand_in();
        for design in designs() {
            for intent in Intent::ALL {
                for facing_right in [true, false] {
                    let ours = intent_frame_hd(&design, &base, intent, 1, facing_right, false, 1);
                    let desktop = DesignRenderer::intent_frame(
                        &design,
                        &base,
                        intent,
                        1,
                        facing_right,
                        false,
                    );
                    assert!(
                        ours.rgba_bytes() == desktop.rgba_bytes(),
                        "{intent:?} facing right {facing_right} differs from Desktop's"
                    );
                }
            }
        }
    }

    #[test]
    fn a_fine_frame_is_the_same_frame_drawn_larger_with_its_face_on_the_same_spot() {
        let base = crate::review::stand_in();
        for design in designs() {
            for detail in 2..=MAX_DETAIL {
                let fine = intent_frame_hd(&design, &base, Intent::Idle, 0, true, true, detail);
                assert_eq!(fine.width(), FRAME_SIZE * detail);
                assert_eq!(fine.height(), FRAME_SIZE * detail);
                let coarse =
                    DesignRenderer::intent_frame(&design, &base, Intent::Idle, 0, true, true);
                let (left, top, right, bottom) = coarse.alpha_bounds().unwrap();
                let (fl, ft, fr, fb) = fine.alpha_bounds().unwrap();
                let d = detail;
                // The same figure in the same place, give or take a fine pixel at its edges.
                assert!(
                    fl / d <= left + 1 && left <= fl / d + 1,
                    "left {fl} vs {left}"
                );
                assert!(ft / d <= top + 1 && top <= ft / d + 1, "top {ft} vs {top}");
                assert!(
                    fr / d + 1 >= right && fr / d <= right + 1,
                    "right {fr} vs {right}"
                );
                assert!(
                    fb / d + 1 >= bottom && fb / d <= bottom + 1,
                    "bottom {fb} vs {bottom}"
                );
            }
        }
    }
}
