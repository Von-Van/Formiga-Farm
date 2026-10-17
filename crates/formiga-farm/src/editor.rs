//! The design on the workbench, and everything that can be done to it: every change can be
//! undone, a slider dragged is one change rather than a hundred, a layer can be put back as it
//! started or rolled at random on its own, and parts a body has no room for are set aside rather
//! than lost.

use formiga_core::{
    BodyPlan, BrowStyle, CheekStyle, CreatureDesign, EarStyle, EyeShape, FaceGenome,
    HighlightStyle, MouthStyle, PupilStyle,
};
use formiga_forms::{
    Coat, Design, Form, Ink, MAX_MARKINGS, Marking, MarkingKind, Part, Plan, STEPS, Sculpt, Shape,
    Slot, Treatment,
};

/// How many changes can be undone.
pub const HISTORY: usize = 200;

/// The four layers of a design, each with its own page in the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    Form,
    Parts,
    Finish,
    Face,
}

impl Layer {
    pub const ALL: [Self; 4] = [Self::Form, Self::Parts, Self::Finish, Self::Face];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Form => "Form",
            Self::Parts => "Parts",
            Self::Finish => "Finish",
            Self::Face => "Face",
        }
    }
}

/// A small, fixed random number generator: the same seed always rolls the same design.
#[derive(Clone, Copy, Debug)]
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u32) -> Self {
        Self(u64::from(seed) ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number from `low` to `high`, both included.
    pub fn range(&mut self, low: u32, high: u32) -> u32 {
        low + (self.next() % u64::from(high - low + 1)) as u32
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.range(0, 99) < percent
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.range(0, items.len() as u32 - 1) as usize]
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
}

/// One state of the workbench that undo goes back to: the design, and the parts set aside from
/// it, so that undoing a change of body takes back what it set aside along with it.
type Step = (Design, Vec<Part>);

pub struct Editor {
    design: Design,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The change in progress, if a slider is being dragged: further changes under the same key
    /// are part of it, not new ones.
    gesture: Option<String>,
    /// How the creature looked when the session opened, for before and after; for a new design,
    /// what it was started from.
    pub arrival: Design,
    /// What it was last started from: a preset, a draft, or the creature itself.
    pub start: Design,
    /// The preset it was last started from, for its lineage.
    pub preset: Option<String>,
    /// Parts the body has no slot for, kept for if it changes back. Undone and redone with the
    /// design.
    set_aside: Vec<Part>,
    /// Bumped on every change, so pictures of the design know when to be redrawn.
    pub changes: u64,
}

impl Editor {
    pub fn new(design: Design, preset: Option<String>) -> Self {
        let design = design.normalized();
        Self {
            arrival: design.clone(),
            start: design.clone(),
            design,
            undo: Vec::new(),
            redo: Vec::new(),
            gesture: None,
            preset,
            set_aside: Vec::new(),
            changes: 0,
        }
    }

    pub fn design(&self) -> &Design {
        &self.design
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Change the design. `key` names a gesture (a slider being dragged, say): changes with the
    /// same key, one after another, undo as one.
    pub fn edit(&mut self, key: Option<&str>, change: impl FnOnce(&mut Design)) {
        self.change(key, |design, _| change(design));
    }

    /// Change the design and the parts set aside from it together, as one change.
    fn change(&mut self, key: Option<&str>, change: impl FnOnce(&mut Design, &mut Vec<Part>)) {
        let mut next = self.design.clone();
        let mut set_aside = self.set_aside.clone();
        change(&mut next, &mut set_aside);
        let next = next.normalized();
        if next == self.design && set_aside == self.set_aside {
            return;
        }
        let continuing = key.is_some() && self.gesture.as_deref() == key;
        if !continuing {
            self.undo
                .push((self.design.clone(), self.set_aside.clone()));
            if self.undo.len() > HISTORY {
                self.undo.remove(0);
            }
        }
        self.gesture = key.map(str::to_owned);
        self.redo.clear();
        self.design = next;
        self.set_aside = set_aside;
        self.changes += 1;
    }

    /// The workbench as it is, swapped for `step`.
    fn swap(&mut self, (design, set_aside): Step) -> Step {
        (
            std::mem::replace(&mut self.design, design),
            std::mem::replace(&mut self.set_aside, set_aside),
        )
    }

    /// The gesture in progress is over: the next change is a new one.
    pub fn settle(&mut self) {
        self.gesture = None;
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            let now = self.swap(previous);
            self.redo.push(now);
            self.gesture = None;
            self.changes += 1;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let now = self.swap(next);
            self.undo.push(now);
            self.gesture = None;
            self.changes += 1;
        }
    }

    /// Start again from `design`, as one change that can be undone.
    pub fn start_from(&mut self, design: Design, preset: Option<String>) {
        let design = design.normalized();
        self.start = design.clone();
        self.preset = preset;
        self.change(None, |d, set_aside| {
            *d = design;
            set_aside.clear();
        });
    }

    /// Put the design back as it was when the session opened.
    pub fn restore_arrival(&mut self) {
        let arrival = self.arrival.clone();
        self.change(None, |d, set_aside| {
            *d = arrival;
            set_aside.clear();
        });
    }

    /// Put back the design it was last started from.
    pub fn reset_to_start(&mut self) {
        let start = self.start.clone();
        self.change(None, |d, set_aside| {
            *d = start;
            set_aside.clear();
        });
    }

    /// Put one layer back as it was started, leaving the rest. Where the design has since
    /// changed from one kind of form to another, the layer cannot be taken on its own, and the
    /// whole start comes back.
    pub fn reset_layer(&mut self, layer: Layer) {
        let start = self.start.clone();
        self.edit(None, |d| match (&mut d.form, &start.form) {
            (Form::Sculpted { sculpt }, Form::Sculpted { sculpt: from }) => match layer {
                Layer::Form => {
                    sculpt.plan = from.plan;
                    sculpt.shape = from.shape;
                }
                Layer::Parts => sculpt.parts = from.parts.clone(),
                Layer::Finish => {
                    sculpt.coat = from.coat;
                    sculpt.markings = from.markings.clone();
                }
                Layer::Face => {
                    sculpt.face_template = from.face_template;
                    d.face = start.face;
                }
            },
            (Form::Companion { recipe }, Form::Companion { recipe: from }) => match layer {
                Layer::Form => {
                    recipe.body = from.body;
                    recipe.width = from.width;
                    recipe.height = from.height;
                    recipe.head = from.head;
                    recipe.legs = from.legs;
                }
                Layer::Parts => {
                    recipe.ears = from.ears;
                    recipe.ear_size = from.ear_size;
                    recipe.tail = from.tail;
                    recipe.muzzle = from.muzzle;
                    recipe.classic = from.classic;
                    recipe.details = from.details;
                }
                Layer::Finish => {
                    recipe.coat = from.coat;
                    recipe.accent = from.accent;
                    recipe.marking = from.marking;
                    recipe.details.belly_color = from.details.belly_color;
                    recipe.details.tip_color = from.details.tip_color;
                }
                Layer::Face => {
                    recipe.face_template = from.face_template;
                    recipe.archetype = from.archetype;
                    d.face = start.face;
                }
            },
            _ => *d = start.clone(),
        });
    }

    /// Put a sculpted form on another plan: parts the new plan has no slot for are set aside,
    /// and any set aside earlier that fit it again come back. A companion or an original look
    /// becomes a sculpted form in its colours.
    pub fn set_plan(&mut self, plan: Plan) {
        self.change(None, |d, set_aside| {
            let sculpt = match &d.form {
                Form::Sculpted { sculpt } => {
                    let mut sculpt = sculpt.clone();
                    sculpt.plan = plan;
                    sculpt
                }
                Form::Companion { recipe } => {
                    let mut sculpt = Sculpt::starter(plan);
                    sculpt.coat.primary = recipe.coat;
                    sculpt.coat.secondary = shade(recipe.coat);
                    sculpt.coat.accent = recipe.accent;
                    sculpt.face_template = recipe.face_template.max(1);
                    sculpt
                }
                Form::Original => Sculpt::starter(plan),
            };
            let mut sculpt = sculpt;
            let (fits, unfit): (Vec<Part>, Vec<Part>) = sculpt
                .parts
                .iter()
                .partition(|part| plan.has(part.kind.slot()));
            sculpt.parts = fits;
            let mut back = Vec::new();
            set_aside.retain(|part| {
                let slot = part.kind.slot();
                if plan.has(slot) && sculpt.part(slot).is_none() {
                    back.push(*part);
                    false
                } else {
                    true
                }
            });
            for part in back {
                sculpt.fit(part);
            }
            for part in unfit {
                set_aside.retain(|kept| kept.kind.slot() != part.kind.slot());
                set_aside.push(part);
            }
            d.form = Form::Sculpted { sculpt };
        });
    }

    /// Put the design on one of Desktop's companion plans, as a recipe.
    pub fn set_companion(&mut self, body: BodyPlan) {
        self.edit(None, |d| {
            let mut recipe = match &d.form {
                Form::Companion { recipe } => *recipe,
                other @ Form::Sculpted { .. } => Design {
                    form: other.clone(),
                    face: d.face,
                }
                .fallback_recipe(None)
                .expect("a sculpted form has a nearest recipe"),
                Form::Original => CreatureDesign::generated([5; 32], 0, None),
            };
            recipe.body = body;
            if recipe.archetype == 0 {
                recipe.archetype = formiga_core::BodyArchetype::for_body(body, false).number();
                recipe.face_template = recipe.face_template.max(1);
            }
            d.form = Form::Companion { recipe };
        });
    }

    pub fn set_aside(&self) -> &[Part] {
        &self.set_aside
    }

    /// Roll one layer at random, leaving the rest. A plan is never changed by rolling parts or
    /// colours, and the design keeps the values rolled, not the roll.
    pub fn randomize(&mut self, layer: Layer, seed: u32) {
        let mut dice = Dice::new(seed);
        self.edit(None, |d| match &mut d.form {
            Form::Sculpted { sculpt } => match layer {
                Layer::Form => roll_shape(&mut sculpt.shape, &mut dice),
                Layer::Parts => roll_parts(sculpt, &mut dice),
                Layer::Finish => roll_finish(sculpt, &mut dice),
                Layer::Face => {
                    sculpt.face_template =
                        dice.range(1, u32::from(formiga_core::FACE_TEMPLATES)) as u8;
                    d.face = roll_face(&mut dice);
                }
            },
            Form::Companion { recipe } => {
                roll_recipe(recipe, layer, &mut dice);
                if layer == Layer::Face {
                    d.face = roll_face(&mut dice);
                }
            }
            Form::Original => {
                if layer == Layer::Face {
                    d.face = roll_face(&mut dice);
                }
            }
        });
    }
}

fn shade(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| (u16::from(c) * 2 / 3) as u8)
}

fn roll_shape(shape: &mut Shape, dice: &mut Dice) {
    for dimension in formiga_forms::Dimension::ALL {
        shape.set(dimension, dice.range(2, STEPS as u32 - 2) as u8);
    }
}

fn roll_parts(sculpt: &mut Sculpt, dice: &mut Dice) {
    let plan = sculpt.plan;
    let keep_shell = sculpt.part(Slot::Shell).copied();
    let mut parts = Vec::new();
    for slot in Slot::ALL {
        if !plan.has(slot) {
            continue;
        }
        // The parts that make a plan read as itself turn up more often on it.
        let likely = match (plan, slot) {
            (Plan::Floater, Slot::Tail | Slot::Fins) => 95,
            (Plan::Crawler, Slot::Shell) => 100,
            (Plan::Percher, Slot::Wings | Slot::Snout | Slot::Feet) => 95,
            (_, Slot::Ears | Slot::Snout | Slot::Tail | Slot::Feet) => 75,
            (_, Slot::Horns | Slot::Mane | Slot::Back) => 25,
            (_, Slot::Tusks | Slot::Wings) => 12,
            _ => 40,
        };
        if !dice.chance(likely) {
            continue;
        }
        let kinds: Vec<_> = slot.kinds().collect();
        let kind = dice.pick(&kinds);
        let ink = match slot {
            Slot::Horns | Slot::Tusks | Slot::Feet if dice.chance(60) => Ink::Feature,
            Slot::Snout if dice.chance(50) => Ink::Underside,
            Slot::Mane | Slot::Shell if dice.chance(60) => Ink::Secondary,
            _ if dice.chance(15) => Ink::Accent,
            _ => Ink::Primary,
        };
        parts.push(Part::new(kind, ink).sized(dice.range(3, 7) as u8));
    }
    if plan == Plan::Crawler
        && let Some(shell) = keep_shell
        && !parts.iter().any(|p| p.kind.slot() == Slot::Shell)
    {
        parts.push(shell);
    }
    sculpt.parts.clear();
    for part in parts {
        sculpt.fit(part);
    }
}

/// A colour from hue, saturation and lightness.
fn hsl(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    formiga_core::hsl(hue, saturation, lightness)
}

fn roll_finish(sculpt: &mut Sculpt, dice: &mut Dice) {
    let hue = dice.unit() * 360.0;
    let natural = dice.chance(50);
    let (saturation, lightness) = if natural {
        (0.15 + dice.unit() * 0.35, 0.4 + dice.unit() * 0.35)
    } else {
        (0.35 + dice.unit() * 0.35, 0.55 + dice.unit() * 0.25)
    };
    let away = dice.pick(&[0.0_f32, 30.0, 180.0, 150.0, 210.0]);
    sculpt.coat = Coat {
        treatment: dice.pick(&Treatment::ALL),
        primary: hsl(hue, saturation, lightness),
        secondary: hsl(hue + away, saturation + 0.1, (lightness - 0.25).max(0.15)),
        underside: hsl(hue + 20.0, 0.3, 0.9),
        accent: hsl(hue + 180.0 + dice.unit() * 60.0, 0.55, 0.72),
        feature: hsl(40.0, 0.4, 0.85),
        eyes: if dice.chance(80) {
            [0x20, 0x1b, 0x29]
        } else {
            hsl(dice.unit() * 360.0, 0.4, 0.3)
        },
        underside_reach: dice.range(0, 8) as u8,
    };
    sculpt.markings.clear();
    let count = dice.pick(&[0, 1, 1, 1, 2]);
    for _ in 0..count.min(MAX_MARKINGS) {
        let kind = dice.pick(&MarkingKind::ALL);
        if sculpt.markings.iter().any(|m| m.kind == kind) {
            continue;
        }
        let color = if dice.chance(50) {
            sculpt.coat.secondary
        } else {
            hsl(
                hue + dice.pick(&[0.0, 180.0, 60.0]),
                0.3,
                if dice.chance(50) { 0.25 } else { 0.85 },
            )
        };
        sculpt.markings.push(Marking {
            amount: dice.range(3, 8) as u8,
            size: dice.range(2, 7) as u8,
            layout: dice.range(0, 255) as u8,
            ..Marking::new(kind, color)
        });
    }
}

fn roll_face(dice: &mut Dice) -> FaceGenome {
    FaceGenome {
        eye_shape: dice.pick(&[
            EyeShape::Round,
            EyeShape::Round,
            EyeShape::Tall,
            EyeShape::SoftSquare,
        ]),
        eye_size: dice.range(1, 2) as u8,
        eye_spacing: dice.range(4, 7) as u8,
        vertical_offset: dice.range(0, 2) as i8 - 1,
        pupil_style: dice.pick(&[PupilStyle::Dot, PupilStyle::Wide, PupilStyle::Spark]),
        highlight_style: dice.pick(&[
            HighlightStyle::Single,
            HighlightStyle::Double,
            HighlightStyle::Diagonal,
        ]),
        brow_style: dice.pick(&[
            BrowStyle::None,
            BrowStyle::None,
            BrowStyle::Soft,
            BrowStyle::Bold,
        ]),
        mouth_style: dice.pick(&[
            MouthStyle::Tiny,
            MouthStyle::Smile,
            MouthStyle::Cat,
            MouthStyle::Beak,
        ]),
        cheek_style: dice.pick(&[CheekStyle::None, CheekStyle::Dots, CheekStyle::Blush]),
    }
}

fn roll_recipe(recipe: &mut CreatureDesign, layer: Layer, dice: &mut Dice) {
    match layer {
        Layer::Form => {
            recipe.width = dice.range(8, 12) as u8;
            recipe.height = dice.range(7, 11) as u8;
            recipe.head = dice.range(7, 9) as u8;
            recipe.legs = dice.range(3, 6) as u8;
        }
        Layer::Parts => {
            recipe.ears = dice.pick(&EarStyle::ALL);
            recipe.ear_size = dice.range(3, 7) as u8;
            recipe.tail = dice.range(0, 4) as u8;
            recipe.muzzle = dice.range(0, 2) as u8;
            recipe.details.horns = if dice.chance(20) {
                dice.range(1, 2) as u8
            } else {
                0
            };
            recipe.details.wings = if dice.chance(15) {
                dice.range(1, 2) as u8
            } else {
                0
            };
        }
        Layer::Finish => {
            let hue = dice.unit() * 360.0;
            recipe.coat = hsl(hue, 0.3 + dice.unit() * 0.3, 0.62 + dice.unit() * 0.18);
            recipe.accent = hsl(hue + dice.pick(&[35.0, 180.0, 120.0]), 0.5, 0.7);
            recipe.marking = dice.range(0, 6) as u8;
        }
        Layer::Face => {
            recipe.face_template = dice.range(1, u32::from(formiga_core::FACE_TEMPLATES)) as u8;
            if recipe.archetype == 0 {
                recipe.archetype =
                    formiga_core::BodyArchetype::for_body(recipe.body, false).number();
            }
        }
    }
    *recipe = recipe.bounded();
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_forms::PartKind;

    fn panda() -> Design {
        crate::presets::find("animal.giant_panda").unwrap().design
    }

    #[test]
    fn a_dragged_slider_undoes_as_one_change() {
        let mut editor = Editor::new(panda(), None);
        for value in 0..=10 {
            editor.edit(Some("head"), |d| {
                if let Form::Sculpted { sculpt } = &mut d.form {
                    sculpt.shape.head = value;
                }
            });
        }
        editor.settle();
        editor.edit(Some("head"), |d| {
            if let Form::Sculpted { sculpt } = &mut d.form {
                sculpt.shape.head = 3;
            }
        });
        editor.undo();
        assert_eq!(editor.design().form.sculpt().unwrap().shape.head, 10);
        editor.undo();
        assert_eq!(*editor.design(), panda());
        assert!(!editor.can_undo());
        editor.redo();
        editor.redo();
        assert_eq!(editor.design().form.sculpt().unwrap().shape.head, 3);
    }

    #[test]
    fn parts_a_plan_has_no_room_for_are_set_aside_and_come_back() {
        let mut editor = Editor::new(panda(), None);
        editor.set_plan(Plan::Floater);
        let sculpt = editor.design().form.sculpt().unwrap().clone();
        assert!(sculpt.part(Slot::Feet).is_none(), "a floater has no feet");
        assert!(
            editor
                .set_aside()
                .iter()
                .any(|p| p.kind == PartKind::FeetPaws)
        );
        editor.set_plan(Plan::CompactQuadruped);
        assert_eq!(
            editor
                .design()
                .form
                .sculpt()
                .unwrap()
                .part(Slot::Feet)
                .map(|p| p.kind),
            Some(PartKind::FeetPaws)
        );
        assert!(editor.set_aside().is_empty());
    }

    #[test]
    fn undoing_a_change_of_body_puts_back_what_was_set_aside_as_it_was() {
        let paws = |editor: &Editor| {
            editor
                .design()
                .form
                .sculpt()
                .unwrap()
                .part(Slot::Feet)
                .map(|p| p.kind)
        };
        // Undone, the floater's set-aside paws are back on the body, and not also said to be
        // set aside; redone, they are set aside again.
        let mut editor = Editor::new(panda(), None);
        editor.set_plan(Plan::Floater);
        editor.undo();
        assert_eq!(paws(&editor), Some(PartKind::FeetPaws));
        assert!(editor.set_aside().is_empty());
        editor.redo();
        assert_eq!(paws(&editor), None);
        assert_eq!(editor.set_aside().len(), 1);

        // Starting again clears what was set aside, and undoing that brings it back, so the
        // paws still return when the body does.
        let mut editor = Editor::new(panda(), None);
        editor.set_plan(Plan::Floater);
        editor.restore_arrival();
        editor.undo();
        editor.set_plan(Plan::CompactQuadruped);
        assert_eq!(paws(&editor), Some(PartKind::FeetPaws));

        // Feet taken off on purpose stay off when the body changes again.
        let mut editor = Editor::new(panda(), None);
        editor.set_plan(Plan::Floater);
        editor.undo();
        editor.edit(None, |d| {
            if let Form::Sculpted { sculpt } = &mut d.form {
                sculpt.remove(Slot::Feet);
            }
        });
        editor.set_plan(Plan::TallQuadruped);
        assert_eq!(paws(&editor), None);
    }

    #[test]
    fn rolling_one_layer_leaves_the_others_and_never_changes_the_plan() {
        let mut editor = Editor::new(panda(), None);
        let before = editor.design().form.sculpt().unwrap().clone();
        for seed in 0..50 {
            editor.randomize(Layer::Finish, seed);
            let after = editor.design().form.sculpt().unwrap();
            assert_eq!(after.plan, before.plan);
            assert_eq!(after.shape, before.shape);
            assert_eq!(after.parts, before.parts);
            assert_eq!(editor.design().validate(), Ok(()));
        }
        for seed in 0..50 {
            editor.randomize(Layer::Parts, seed);
            assert_eq!(editor.design().form.sculpt().unwrap().plan, before.plan);
            assert_eq!(editor.design().validate(), Ok(()));
        }
        // The same seed rolls the same design.
        let mut a = Editor::new(panda(), None);
        let mut b = Editor::new(panda(), None);
        a.randomize(Layer::Finish, 77);
        b.randomize(Layer::Finish, 77);
        assert_eq!(a.design(), b.design());
    }

    #[test]
    fn a_layer_can_be_put_back_on_its_own() {
        let mut editor = Editor::new(panda(), None);
        editor.randomize(Layer::Finish, 3);
        editor.randomize(Layer::Form, 3);
        editor.reset_layer(Layer::Finish);
        let sculpt = editor.design().form.sculpt().unwrap();
        let start = panda();
        let from = start.form.sculpt().unwrap();
        assert_eq!(sculpt.coat, from.coat);
        assert_ne!(sculpt.shape, from.shape, "the form is still rolled");
    }

    #[test]
    fn a_companion_becomes_a_sculpt_in_its_own_colours_and_back() {
        let preset = crate::presets::find("form.round").unwrap();
        let mut editor = Editor::new(preset.design.clone(), None);
        let recipe = preset.design.form.recipe().unwrap();
        editor.set_plan(Plan::Percher);
        assert_eq!(
            editor.design().form.sculpt().unwrap().coat.primary,
            recipe.coat
        );
        editor.set_companion(BodyPlan::Upright);
        let back = editor.design().form.recipe().unwrap();
        assert_eq!(back.body, BodyPlan::Upright);
        assert_eq!(back.coat, recipe.coat);
        assert_eq!(editor.design().validate(), Ok(()));
    }
}
