//! The renderer, held to what Desktop needs of it: a creature that keeps its body draws exactly
//! as Desktop draws it today, and a sculpted form draws every clip Desktop plays, inside its
//! frame, standing (or hovering) where it should.

use formiga_art::{
    BodyClip, CreatureRenderer, ExpressionKind, EyelidPose, FRAME_SIZE, FaceRenderState,
    GazeDirection,
};
use formiga_core::ActionKind;
use formiga_forms::{Design, DesignRenderer, Form, Plan, Sculpt, plain_face};

fn face() -> FaceRenderState {
    FaceRenderState {
        expression: ExpressionKind::Content,
        eyelids: EyelidPose::Open,
        gaze: GazeDirection::new(0, 0),
    }
}

fn sculpted(plan: Plan) -> Design {
    Design {
        form: Form::Sculpted {
            sculpt: Sculpt::starter(plan),
        },
        face: plain_face(),
    }
}

fn clips() -> Vec<BodyClip> {
    BodyClip::baked().collect()
}

/// Every creature of Desktop's sample colony (every recipe edition, and one from before
/// recipes) draws pixel for pixel as Desktop's own renderer draws it, in every clip, both ways
/// round: taking a creature's design into Farm and back changes nothing about how it looks.
#[test]
fn a_creature_that_keeps_its_body_draws_exactly_as_desktop_draws_it() {
    let colony = formiga_travel::sample::colony(3);
    for creature in &colony.creatures {
        let genome = &creature.appearance;
        let design = Design::of(genome, None);
        assert_eq!(design.genome(genome), *genome, "{}", creature.name);
        for clip in clips() {
            for frame in [0, 1] {
                for facing_right in [true, false] {
                    assert_eq!(
                        DesignRenderer::frame(
                            &design,
                            genome,
                            clip,
                            frame,
                            facing_right,
                            false,
                            face()
                        ),
                        CreatureRenderer::render_composited_frame(
                            genome,
                            clip,
                            frame,
                            facing_right,
                            false,
                            face()
                        ),
                        "{} {clip:?} {frame}",
                        creature.name
                    );
                }
            }
        }
        assert_eq!(
            DesignRenderer::resting_baseline(&design, genome, false),
            CreatureRenderer::resting_baseline(genome, false)
        );
    }
}

/// Every plan draws every clip Desktop bakes, every frame of it, inside the frame with a pixel
/// to spare, as the atlas needs.
#[test]
fn every_plan_draws_every_clip_inside_its_frame() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    for plan in Plan::ALL {
        let design = sculpted(plan);
        for clip in clips() {
            let frames = formiga_art::AnimationSpec::for_clip(clip).frames;
            for frame in 0..frames {
                let body = DesignRenderer::body_frame(&design, &base, clip, frame, false);
                let (left, top, right, bottom) = body
                    .canvas
                    .alpha_bounds()
                    .unwrap_or_else(|| panic!("{plan:?} {clip:?} {frame} draws nothing"));
                assert!(
                    left >= 1 && top >= 1 && right <= FRAME_SIZE - 2 && bottom <= FRAME_SIZE - 2,
                    "{plan:?} {clip:?} {frame} reaches the edge: {left} {top} {right} {bottom}"
                );
                let face = body.face_anchor;
                assert!(
                    (8..=40).contains(&face.x) && (8..=40).contains(&face.y),
                    "{plan:?} {clip:?} the face would leave the frame at {face:?}"
                );
            }
        }
    }
}

/// A whale does not walk on invisible legs: at rest and moving it hovers above the ground it
/// stands for, and it only comes down to rest.
#[test]
fn a_floater_hovers_and_never_walks_on_legs_it_does_not_have() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    let design = sculpted(Plan::Floater);
    let ground = (FRAME_SIZE - 1 - DesignRenderer::resting_baseline(&design, &base, false)) as i32;
    for action in [ActionKind::Idle, ActionKind::Traverse, ActionKind::Follow] {
        for frame in 0..formiga_art::AnimationSpec::for_action(action).frames {
            let body = DesignRenderer::body_frame(&design, &base, action, frame, false);
            let bottom = body.canvas.alpha_bounds().unwrap().3 as i32;
            assert!(
                bottom < ground - 1,
                "{action:?} {frame} touches down at {bottom}, ground {ground}"
            );
        }
    }
    let resting = DesignRenderer::body_frame(&design, &base, ActionKind::Sleep, 0, false);
    assert!(
        resting.canvas.alpha_bounds().unwrap().3 as i32 >= ground - 1,
        "it settles to rest"
    );
}

/// A four-legged form stands with its feet on the ground row, so Desktop can stand it on a
/// window or the dock exactly as it stands a companion.
#[test]
fn walkers_stand_on_their_ground_row() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    for plan in [
        Plan::CompactQuadruped,
        Plan::LargeQuadruped,
        Plan::TallQuadruped,
        Plan::Upright,
        Plan::Crawler,
        Plan::Percher,
    ] {
        let design = sculpted(plan);
        let ground = FRAME_SIZE - 1 - DesignRenderer::resting_baseline(&design, &base, false);
        let body = DesignRenderer::body_frame(&design, &base, ActionKind::Idle, 0, false);
        let bottom = body.canvas.alpha_bounds().unwrap().3;
        assert!(
            bottom.abs_diff(ground) <= 1,
            "{plan:?}: feet at {bottom}, ground {ground}"
        );
    }
}

/// The same design always draws the same frame, and reduced motion holds every clip still.
#[test]
fn drawing_is_a_pure_function_of_the_design_and_reduced_motion_holds_still() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    for plan in Plan::ALL {
        let design = sculpted(plan);
        let a = DesignRenderer::frame(&design, &base, ActionKind::Traverse, 3, true, false, face());
        let b = DesignRenderer::frame(&design, &base, ActionKind::Traverse, 3, true, false, face());
        assert_eq!(a, b);
        let still: Vec<_> = (0..6)
            .map(|frame| {
                DesignRenderer::frame(
                    &design,
                    &base,
                    ActionKind::Traverse,
                    frame,
                    true,
                    true,
                    face(),
                )
            })
            .collect();
        assert!(
            still.windows(2).all(|pair| pair[0] == pair[1]),
            "{plan:?} moves under reduced motion"
        );
    }
}

/// Facing left is facing right in a mirror, face and all, as Desktop draws a companion.
#[test]
fn facing_left_is_the_mirror_of_facing_right() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    let design = sculpted(Plan::Percher);
    let right = DesignRenderer::frame(&design, &base, ActionKind::Idle, 0, true, false, face());
    let mut left = DesignRenderer::frame(&design, &base, ActionKind::Idle, 0, false, false, face());
    left.mirror_horizontal();
    assert_eq!(left, right);
}

/// Different plans make different silhouettes, not one body in different paint.
#[test]
fn every_plan_has_a_silhouette_of_its_own() {
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    let silhouettes: Vec<Vec<bool>> = Plan::ALL
        .into_iter()
        .map(|plan| {
            DesignRenderer::body_frame(&sculpted(plan), &base, ActionKind::Idle, 0, true)
                .alpha_mask
                .pixels
        })
        .collect();
    for (i, a) in silhouettes.iter().enumerate() {
        for (j, b) in silhouettes.iter().enumerate().skip(i + 1) {
            let differ = a.iter().zip(b).filter(|(x, y)| x != y).count();
            assert!(
                differ > 120,
                "{:?} and {:?} differ by only {differ} pixels",
                Plan::ALL[i],
                Plan::ALL[j]
            );
        }
    }
}
