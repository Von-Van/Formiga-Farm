//! What the owner should know about a design before it is applied, said before rather than
//! after: whether Desktop can keep it at all, whether the creature is home to be changed,
//! whether its eyes still show, and what has been set aside.

use crate::paint::contrast;
use formiga_core::AppearanceGenome;
use formiga_farm_contract::{Availability, FarmCapability, FarmMode, FarmSnapshot};
use formiga_forms::{Design, DesignRenderer, Form, Intent, MarkingKind, Part, Plan, Slot};

/// How much a notice matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Worth knowing.
    Note,
    /// Worth a second look, but nothing stops it.
    Warning,
    /// The design cannot be applied until this is so.
    Blocks,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub level: Level,
    pub text: String,
}

fn notice(level: Level, text: impl Into<String>) -> Notice {
    Notice {
        level,
        text: text.into(),
    }
}

/// Everything worth saying about `design`, most important first.
pub fn check(
    design: &Design,
    snapshot: &FarmSnapshot,
    base: &AppearanceGenome,
    set_aside: &[Part],
) -> Vec<Notice> {
    let mut notices = Vec::new();
    if let Err(error) = design.validate() {
        notices.push(notice(Level::Blocks, error.to_string()));
    }
    if let Some(sculpt) = design.form.sculpt() {
        if !snapshot.offers(FarmCapability::SculptedForms) {
            notices.push(notice(
                Level::Blocks,
                "This Desktop can only keep its own companion shapes. Choose one of them under Form, or save this design for later.",
            ));
        } else if !snapshot.renderer.plans.contains(&sculpt.plan) {
            notices.push(notice(
                Level::Blocks,
                format!(
                    "This Desktop cannot draw a {} body yet.",
                    sculpt.plan.label().to_lowercase()
                ),
            ));
        }
    }
    match &snapshot.mode {
        FarmMode::EditExisting { creature } => {
            if creature.availability != Availability::Available {
                notices.push(notice(
                    Level::Blocks,
                    format!(
                        "{} is out with another Formiga app. Keep drafting; you can apply this when they are back.",
                        creature.name
                    ),
                ));
            }
            if !snapshot.offers(FarmCapability::EditExisting) {
                notices.push(notice(
                    Level::Blocks,
                    "This Desktop does not take changes to a companion's look.",
                ));
            }
            if matches!(design.form, Form::Original) && base.design.is_some() {
                notices.push(notice(
                    Level::Blocks,
                    "Only a companion that already has its original look can keep it.",
                ));
            }
            if let formiga_travel::TravelRole::Mini { .. } = creature.role {
                notices.push(notice(
                    Level::Note,
                    format!(
                        "{} is a little one. Their grown-up keeps their own look.",
                        creature.name
                    ),
                ));
            }
        }
        FarmMode::Create { .. } => {
            if !snapshot.offers(FarmCapability::CreateNew) {
                notices.push(notice(
                    Level::Blocks,
                    "This Desktop does not take new companions from Farm.",
                ));
            }
            if matches!(design.form, Form::Original) {
                notices.push(notice(
                    Level::Blocks,
                    "A new companion needs a body: choose one under Form.",
                ));
            }
        }
    }
    // Every intent must draw: a body the renderer cannot carry through all of them is not one
    // Desktop could keep.
    for intent in Intent::ALL {
        let frame = DesignRenderer::body_frame(design, base, intent.clip(), 0, false);
        if frame.canvas.alpha_bounds().is_none() {
            notices.push(notice(
                Level::Blocks,
                format!(
                    "It cannot be drawn when it is {}.",
                    intent.label().to_lowercase()
                ),
            ));
        }
    }
    if let Some(sculpt) = design.form.sculpt() {
        // The eyes against whatever they sit on.
        let around = sculpt
            .markings
            .iter()
            .rev()
            .find(|m| matches!(m.kind, MarkingKind::EyePatches | MarkingKind::Cap))
            .map_or(sculpt.coat.primary, |m| m.color);
        let soft = around.map(|c| ((u16::from(c) * 3 + 220) / 4) as u8);
        if contrast(sculpt.coat.eyes, soft) < 1.8 {
            notices.push(notice(
                Level::Warning,
                "Its eyes are hard to see against its face. Try lighter markings round them, or another eye colour.",
            ));
        }
        if sculpt.plan == Plan::Floater
            && sculpt.part(Slot::Tail).is_none()
            && sculpt.part(Slot::Fins).is_none()
        {
            notices.push(notice(
                Level::Note,
                "A floater with no tail or fins drifts rather than swims.",
            ));
        }
    }
    if !set_aside.is_empty() {
        let names: Vec<String> = set_aside
            .iter()
            .map(|part| {
                format!(
                    "{} {}",
                    part.kind.label().to_lowercase(),
                    part.kind.slot().label().to_lowercase()
                )
            })
            .collect();
        notices.push(notice(
            Level::Note,
            format!(
                "Set aside for now, since this body has nowhere for them: {}. They come back if the body changes back.",
                names.join(", ")
            ),
        ));
    }
    notices.sort_by_key(|notice| std::cmp::Reverse(notice.level));
    notices
}

/// Whether anything in `notices` stops the design being applied.
pub fn blocked(notices: &[Notice]) -> bool {
    notices.iter().any(|n| n.level == Level::Blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_farm_contract::sample;

    #[test]
    fn every_preset_can_be_applied_in_a_session_that_offers_everything() {
        let snapshot = sample::create_snapshot();
        let base = snapshot.base_genome().unwrap();
        for preset in crate::presets::all() {
            let notices = check(&preset.design, &snapshot, &base, &[]);
            assert!(!blocked(&notices), "{}: {notices:?}", preset.id);
        }
    }

    #[test]
    fn a_desktop_without_sculpts_blocks_a_sculpt_and_says_why() {
        let mut snapshot = sample::create_snapshot();
        snapshot
            .capabilities
            .retain(|c| *c != FarmCapability::SculptedForms);
        let base = snapshot.base_genome().unwrap();
        let design = crate::presets::find("animal.orca").unwrap().design;
        let notices = check(&design, &snapshot, &base, &[]);
        assert!(blocked(&notices));
        let companion = crate::presets::find("form.round").unwrap().design;
        assert!(!blocked(&check(&companion, &snapshot, &base, &[])));
    }

    #[test]
    fn eyes_lost_in_a_dark_face_are_pointed_out() {
        let snapshot = sample::create_snapshot();
        let base = snapshot.base_genome().unwrap();
        let mut design = crate::presets::find("animal.tiger").unwrap().design;
        if let Form::Sculpted { sculpt } = &mut design.form {
            sculpt.coat.primary = [0x10, 0x10, 0x14];
        }
        let notices = check(&design, &snapshot, &base, &[]);
        assert!(
            notices
                .iter()
                .any(|n| n.level == Level::Warning && n.text.contains("eyes"))
        );
    }
}
