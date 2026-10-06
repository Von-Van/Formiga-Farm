//! The starting points that come with Farm: a blank form, every body plan, and twenty animals,
//! each a Formiga's idea of the animal rather than the animal itself. A preset is only ever
//! where a design starts; it carries no trait, no rarity and no advantage, and nothing about a
//! creature remembers which one it began as except the design's own lineage note.

use formiga_core::{
    BrowStyle, CheekStyle, EyeShape, FaceGenome, HighlightStyle, MouthStyle, PupilStyle,
};
use formiga_forms::{
    Coat, Design, Form, Ink, Marking, MarkingKind, Part, PartKind, Plan, Sculpt, Shape, Treatment,
    plain_face,
};

/// Which group a preset is listed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Blank,
    BodyForm,
    Animal,
}

/// One preset.
#[derive(Clone, Debug)]
pub struct Preset {
    /// Its identifier, as a design's lineage note names it: `animal.giant_panda`.
    pub id: &'static str,
    pub name: &'static str,
    pub shelf: Shelf,
    pub design: Design,
}

/// Every preset, in the order the shelves list them.
pub fn all() -> Vec<Preset> {
    let mut presets = vec![Preset {
        id: "blank",
        name: "Blank",
        shelf: Shelf::Blank,
        design: blank(),
    }];
    for plan in formiga_core::BodyPlan::ALL {
        presets.push(Preset {
            id: companion_id(plan),
            name: plan.label(),
            shelf: Shelf::BodyForm,
            design: Design {
                form: Form::Companion {
                    recipe: companion(plan),
                },
                face: plain_face(),
            },
        });
    }
    for plan in Plan::ALL {
        presets.push(Preset {
            id: plan_id(plan),
            name: plan.label(),
            shelf: Shelf::BodyForm,
            design: Design {
                form: Form::Sculpted {
                    sculpt: Sculpt::starter(plan),
                },
                face: plain_face(),
            },
        });
    }
    presets.extend(animals());
    presets
}

pub fn find(id: &str) -> Option<Preset> {
    all().into_iter().find(|preset| preset.id == id)
}

/// A form with nothing on it yet: a plain body on four legs, and a face.
pub fn blank() -> Design {
    let mut sculpt = Sculpt::starter(Plan::CompactQuadruped);
    sculpt.parts.retain(|part| part.kind == PartKind::FeetPaws);
    sculpt.coat = Coat {
        primary: [0xe6, 0xd8, 0xc0],
        secondary: [0xb8, 0xa5, 0x8a],
        underside: [0xf6, 0xee, 0xe0],
        accent: [0xe8, 0x9c, 0x9c],
        feature: [0xf0, 0xe6, 0xd6],
        ..Coat::default()
    };
    Design {
        form: Form::Sculpted { sculpt },
        face: plain_face(),
    }
}

fn companion_id(plan: formiga_core::BodyPlan) -> &'static str {
    use formiga_core::BodyPlan as P;
    match plan {
        P::Round => "form.round",
        P::Upright => "form.upright",
        P::Long => "form.four_pawed",
        P::Winged => "form.winged",
        P::Blob => "form.blob",
    }
}

fn plan_id(plan: Plan) -> &'static str {
    match plan {
        Plan::CompactQuadruped => "form.compact_quadruped",
        Plan::LargeQuadruped => "form.large_quadruped",
        Plan::TallQuadruped => "form.tall_quadruped",
        Plan::Upright => "form.upright_sculpted",
        Plan::Floater => "form.floater",
        Plan::Crawler => "form.crawler",
        Plan::Percher => "form.percher",
    }
}

/// A plain companion recipe on one of Desktop's five plans: the archetype the plan belongs to,
/// in soft colours, with nothing extra.
fn companion(plan: formiga_core::BodyPlan) -> formiga_core::CreatureDesign {
    let archetype = formiga_core::BodyArchetype::for_body(plan, false);
    let mut recipe = formiga_core::CreatureDesign::generated([3; 32], 0, None);
    recipe.body = plan;
    recipe.archetype = archetype.number();
    recipe.face_template = 1;
    recipe.ears = formiga_core::EarStyle::Round;
    recipe.ear_size = 4;
    recipe.tail = if plan == formiga_core::BodyPlan::Long {
        2
    } else {
        1
    };
    recipe.width = 10;
    recipe.height = 9;
    recipe.head = 9;
    recipe.legs = 4;
    recipe.muzzle = 0;
    recipe.marking = 1;
    recipe.coat = [0xe8, 0xc8, 0x9a];
    recipe.accent = [0xe8, 0x92, 0x8a];
    recipe.classic = Default::default();
    recipe.details = Default::default();
    recipe.bounded()
}

/// A face with its features named.
fn face(
    eye_shape: EyeShape,
    eye_size: u8,
    eye_spacing: u8,
    mouth: MouthStyle,
    cheeks: CheekStyle,
) -> FaceGenome {
    FaceGenome {
        eye_shape,
        eye_size,
        eye_spacing,
        vertical_offset: 0,
        pupil_style: PupilStyle::Dot,
        highlight_style: HighlightStyle::Single,
        brow_style: BrowStyle::None,
        mouth_style: mouth,
        cheek_style: cheeks,
    }
}

struct Animal {
    id: &'static str,
    name: &'static str,
    plan: Plan,
    shape: [u8; 8],
    treatment: Treatment,
    /// Primary, secondary, underside, accent, feature, eyes.
    colors: [[u8; 3]; 6],
    underside_reach: u8,
    parts: Vec<Part>,
    markings: Vec<Marking>,
    face_template: u8,
    face: FaceGenome,
}

impl Animal {
    fn preset(self) -> Preset {
        let [head, length, depth, neck, legs, arms, girth, tail] = self.shape;
        let [primary, secondary, underside, accent, feature, eyes] = self.colors;
        let mut sculpt = Sculpt {
            plan: self.plan,
            shape: Shape {
                head,
                length,
                depth,
                neck,
                legs,
                arms,
                girth,
                tail,
            },
            parts: Vec::new(),
            coat: Coat {
                treatment: self.treatment,
                primary,
                secondary,
                underside,
                accent,
                feature,
                eyes,
                underside_reach: self.underside_reach,
            },
            markings: self.markings,
            face_template: self.face_template,
        };
        for part in self.parts {
            sculpt.fit(part);
        }
        let sculpt = sculpt.normalized();
        Preset {
            id: self.id,
            name: self.name,
            shelf: Shelf::Animal,
            design: Design {
                form: Form::Sculpted { sculpt },
                face: self.face,
            },
        }
    }
}

fn part(kind: PartKind, ink: Ink, size: u8) -> Part {
    Part::new(kind, ink).sized(size)
}

fn mark(kind: MarkingKind, color: [u8; 3], amount: u8, size: u8) -> Marking {
    Marking {
        amount,
        size,
        ..Marking::new(kind, color)
    }
}

const EYES: [u8; 3] = [0x20, 0x1b, 0x29];
const BLUSH: [u8; 3] = [0xee, 0x9a, 0xa0];

/// The twenty animals. Their order is the order they are listed in.
pub fn animals() -> Vec<Preset> {
    use Ink::*;
    use MarkingKind as M;
    use PartKind as K;
    use Plan as P;
    use Treatment as T;
    let cat = face(EyeShape::Round, 2, 5, MouthStyle::Cat, CheekStyle::Blush);
    let bear = face(EyeShape::Round, 2, 5, MouthStyle::Tiny, CheekStyle::Blush);
    let gentle = face(EyeShape::Round, 2, 6, MouthStyle::Smile, CheekStyle::Blush);
    let sea = face(EyeShape::Round, 2, 6, MouthStyle::Smile, CheekStyle::Dots);
    let ape = face(EyeShape::Round, 2, 4, MouthStyle::Smile, CheekStyle::None);
    [
        Animal {
            id: "animal.giant_panda",
            name: "Giant panda",
            plan: P::CompactQuadruped,
            shape: [7, 4, 6, 3, 3, 5, 7, 2],
            treatment: T::Fur,
            colors: [
                [0xfa, 0xf8, 0xf2],
                [0x3a, 0x34, 0x3c],
                [0xfa, 0xf8, 0xf2],
                BLUSH,
                [0x50, 0x48, 0x50],
                EYES,
            ],
            underside_reach: 0,
            parts: vec![
                part(K::EarsRound, Secondary, 5),
                part(K::SnoutMuzzle, Underside, 4),
                part(K::TailStub, Primary, 4),
                part(K::FeetPaws, Secondary, 5),
            ],
            markings: vec![
                mark(M::EyePatches, [0x5a, 0x50, 0x5a], 5, 5),
                mark(M::Shoulders, [0x3a, 0x34, 0x3c], 5, 5),
                mark(M::Socks, [0x3a, 0x34, 0x3c], 9, 5),
            ],
            face_template: 2,
            face: bear,
        },
        Animal {
            id: "animal.african_elephant",
            name: "African elephant",
            plan: P::LargeQuadruped,
            shape: [6, 5, 5, 4, 5, 5, 6, 3],
            treatment: T::Plated,
            colors: [
                [0x9c, 0xa0, 0xaa],
                [0x7c, 0x80, 0x8c],
                [0xb4, 0xb8, 0xc0],
                [0xe8, 0xa8, 0xb0],
                [0xf6, 0xee, 0xdc],
                EYES,
            ],
            underside_reach: 2,
            parts: vec![
                part(K::EarsFan, Primary, 9),
                part(K::SnoutTrunk, Primary, 6),
                part(K::TusksLong, Feature, 5),
                part(K::TailTufted, Primary, 3),
                part(K::FeetPads, Primary, 5),
            ],
            markings: vec![],
            face_template: 1,
            face: gentle,
        },
        Animal {
            id: "animal.asian_elephant",
            name: "Asian elephant",
            plan: P::LargeQuadruped,
            shape: [7, 4, 6, 4, 4, 5, 6, 3],
            treatment: T::Plated,
            colors: [
                [0x8e, 0x88, 0x8c],
                [0x70, 0x6a, 0x70],
                [0xa8, 0xa0, 0xa4],
                [0xe8, 0xa8, 0xb0],
                [0xf6, 0xee, 0xdc],
                EYES,
            ],
            underside_reach: 2,
            parts: vec![
                part(K::EarsFan, Primary, 4),
                part(K::SnoutTrunk, Primary, 5),
                part(K::TusksShort, Feature, 3),
                part(K::TailTufted, Primary, 3),
                part(K::FeetPads, Primary, 5),
                part(K::BackHump, Primary, 3),
            ],
            markings: vec![mark(M::Mottle, [0xd8, 0xb0, 0xb0], 2, 3)],
            face_template: 3,
            face: gentle,
        },
        Animal {
            id: "animal.tiger",
            name: "Tiger",
            plan: P::CompactQuadruped,
            shape: [6, 6, 5, 4, 5, 5, 5, 6],
            treatment: T::Fur,
            colors: [
                [0xf0, 0x8c, 0x32],
                [0x3a, 0x2c, 0x2c],
                [0xfc, 0xf6, 0xea],
                BLUSH,
                [0xf4, 0xec, 0xdc],
                EYES,
            ],
            underside_reach: 5,
            parts: vec![
                part(K::EarsRound, Primary, 4),
                part(K::SnoutMuzzle, Underside, 5),
                part(K::TailTaper, Primary, 6),
                part(K::FeetPaws, Primary, 5),
            ],
            markings: vec![mark(M::Stripes, [0x3a, 0x2c, 0x2c], 4, 4)],
            face_template: 2,
            face: cat,
        },
        Animal {
            id: "animal.lion",
            name: "Lion",
            plan: P::CompactQuadruped,
            shape: [6, 6, 5, 4, 5, 5, 5, 6],
            treatment: T::Fur,
            colors: [
                [0xe6, 0xb4, 0x6c],
                [0xa8, 0x5c, 0x2c],
                [0xf8, 0xe8, 0xc8],
                BLUSH,
                [0xf4, 0xec, 0xdc],
                EYES,
            ],
            underside_reach: 4,
            parts: vec![
                part(K::EarsRound, Primary, 3),
                part(K::SnoutMuzzle, Underside, 5),
                part(K::ManeRuff, Secondary, 6),
                part(K::TailTufted, Primary, 7),
                part(K::FeetPaws, Primary, 6),
            ],
            markings: vec![],
            face_template: 2,
            face: cat,
        },
        Animal {
            id: "animal.polar_bear",
            name: "Polar bear",
            plan: P::CompactQuadruped,
            shape: [6, 6, 6, 4, 4, 5, 7, 2],
            treatment: T::Fur,
            colors: [
                [0xf6, 0xf2, 0xe4],
                [0xd8, 0xd2, 0xc0],
                [0xfc, 0xfa, 0xf2],
                BLUSH,
                [0x48, 0x40, 0x48],
                EYES,
            ],
            underside_reach: 0,
            parts: vec![
                part(K::EarsSmall, Primary, 5),
                part(K::SnoutBroad, Underside, 5),
                part(K::TailStub, Primary, 3),
                part(K::FeetBigPaws, Primary, 5),
            ],
            markings: vec![],
            face_template: 1,
            face: bear,
        },
        Animal {
            id: "animal.gray_wolf",
            name: "Gray wolf",
            plan: P::CompactQuadruped,
            shape: [5, 6, 5, 5, 6, 5, 4, 6],
            treatment: T::Fur,
            colors: [
                [0xa8, 0xa8, 0xb0],
                [0x6a, 0x6a, 0x74],
                [0xf2, 0xea, 0xdc],
                BLUSH,
                [0x48, 0x40, 0x48],
                EYES,
            ],
            underside_reach: 6,
            parts: vec![
                part(K::EarsPointed, Primary, 6),
                part(K::SnoutLong, Underside, 5),
                part(K::TailPlume, Primary, 6),
                part(K::FeetPaws, Primary, 5),
            ],
            markings: vec![mark(M::Saddle, [0x6a, 0x6a, 0x74], 5, 5)],
            face_template: 4,
            face: face(EyeShape::Round, 2, 5, MouthStyle::Smile, CheekStyle::None),
        },
        Animal {
            id: "animal.humpback_whale",
            name: "Humpback whale",
            plan: P::Floater,
            shape: [7, 5, 6, 5, 5, 5, 5, 5],
            treatment: T::Smooth,
            colors: [
                [0x50, 0x60, 0x7a],
                [0x3c, 0x48, 0x5c],
                [0xe0, 0xe6, 0xec],
                BLUSH,
                [0xe0, 0xe6, 0xec],
                EYES,
            ],
            underside_reach: 5,
            parts: vec![
                part(K::FinsLong, Underside, 5),
                part(K::TailFluke, Primary, 6),
                part(K::BackHump, Primary, 2),
            ],
            markings: vec![mark(M::Spots, [0xc8, 0xd0, 0xdc], 3, 0)],
            face_template: 1,
            face: sea,
        },
        Animal {
            id: "animal.blue_whale",
            name: "Blue whale",
            plan: P::Floater,
            shape: [5, 9, 4, 5, 5, 5, 5, 6],
            treatment: T::Smooth,
            colors: [
                [0x6c, 0x8c, 0xb4],
                [0x50, 0x6c, 0x94],
                [0xc0, 0xd0, 0xe0],
                BLUSH,
                [0xe0, 0xe6, 0xec],
                EYES,
            ],
            underside_reach: 3,
            parts: vec![
                part(K::FinsPectoral, Primary, 4),
                part(K::TailFluke, Primary, 6),
                part(K::BackTinyDorsal, Primary, 4),
            ],
            markings: vec![mark(M::Mottle, [0xa8, 0xbc, 0xd4], 4, 3)],
            face_template: 3,
            face: sea,
        },
        Animal {
            id: "animal.bottlenose_dolphin",
            name: "Bottlenose dolphin",
            plan: P::Floater,
            shape: [6, 3, 4, 5, 5, 5, 5, 4],
            treatment: T::Smooth,
            colors: [
                [0x92, 0xa2, 0xb4],
                [0x6c, 0x7c, 0x90],
                [0xe6, 0xea, 0xf0],
                BLUSH,
                [0xe0, 0xe6, 0xec],
                EYES,
            ],
            underside_reach: 4,
            parts: vec![
                part(K::SnoutRostrum, Primary, 5),
                part(K::FinsFlippers, Primary, 4),
                part(K::TailFluke, Primary, 4),
                part(K::BackDorsal, Primary, 5),
            ],
            markings: vec![],
            face_template: 1,
            face: sea,
        },
        Animal {
            id: "animal.orca",
            name: "Orca",
            plan: P::Floater,
            shape: [6, 5, 5, 5, 5, 5, 5, 5],
            treatment: T::Smooth,
            colors: [
                [0x2c, 0x2a, 0x34],
                [0x2c, 0x2a, 0x34],
                [0xfa, 0xfa, 0xf6],
                BLUSH,
                [0xfa, 0xfa, 0xf6],
                [0x20, 0x1b, 0x29],
            ],
            underside_reach: 6,
            parts: vec![
                part(K::FinsPectoral, Primary, 5),
                part(K::TailFluke, Primary, 5),
                part(K::BackTallDorsal, Primary, 5),
            ],
            markings: vec![
                mark(M::EyePatches, [0xfa, 0xfa, 0xf6], 4, 4),
                mark(M::Saddle, [0xb0, 0xb0, 0xbc], 2, 5),
            ],
            face_template: 1,
            face: sea,
        },
        Animal {
            id: "animal.gorilla",
            name: "Gorilla",
            plan: P::Upright,
            shape: [6, 7, 6, 4, 4, 8, 8, 0],
            treatment: T::Fur,
            colors: [
                [0x4a, 0x48, 0x52],
                [0x7a, 0x78, 0x84],
                [0x6c, 0x68, 0x72],
                BLUSH,
                [0x3c, 0x38, 0x40],
                EYES,
            ],
            underside_reach: 4,
            parts: vec![
                part(K::EarsSmall, Primary, 4),
                part(K::SnoutBroad, Underside, 5),
                part(K::FeetHands, Primary, 5),
            ],
            markings: vec![mark(M::Mask, [0x8e, 0x84, 0x88], 6, 5)],
            face_template: 1,
            face: ape,
        },
        Animal {
            id: "animal.chimpanzee",
            name: "Chimpanzee",
            plan: P::Upright,
            shape: [6, 4, 5, 4, 4, 7, 5, 0],
            treatment: T::Fur,
            colors: [
                [0x5c, 0x46, 0x3c],
                [0x3e, 0x30, 0x2a],
                [0xe8, 0xcc, 0xac],
                BLUSH,
                [0x3c, 0x30, 0x2c],
                EYES,
            ],
            underside_reach: 0,
            parts: vec![
                part(K::EarsRound, Underside, 6),
                part(K::SnoutMuzzle, Underside, 5),
                part(K::FeetHands, Primary, 5),
            ],
            markings: vec![mark(M::Mask, [0xe8, 0xcc, 0xac], 8, 5)],
            face_template: 2,
            face: ape,
        },
        Animal {
            id: "animal.orangutan",
            name: "Orangutan",
            plan: P::Upright,
            shape: [6, 5, 5, 4, 3, 9, 5, 0],
            treatment: T::Shaggy,
            colors: [
                [0xd4, 0x74, 0x3c],
                [0xa8, 0x50, 0x28],
                [0xe8, 0xa8, 0x7c],
                BLUSH,
                [0x6c, 0x40, 0x30],
                EYES,
            ],
            underside_reach: 0,
            parts: vec![
                part(K::EarsSmall, Primary, 3),
                part(K::SnoutMuzzle, Underside, 4),
                part(K::ManeShaggy, Primary, 3),
                part(K::FeetHands, Primary, 5),
            ],
            markings: vec![mark(M::Mask, [0xe8, 0xa8, 0x7c], 7, 5)],
            face_template: 3,
            face: ape,
        },
        Animal {
            id: "animal.rhinoceros",
            name: "Rhinoceros",
            plan: P::LargeQuadruped,
            shape: [5, 5, 6, 4, 4, 5, 7, 3],
            treatment: T::Plated,
            colors: [
                [0xa8, 0xa2, 0x9c],
                [0x84, 0x7e, 0x78],
                [0xbc, 0xb6, 0xb0],
                [0xe8, 0xa8, 0xb0],
                [0xf0, 0xe4, 0xcc],
                EYES,
            ],
            underside_reach: 2,
            parts: vec![
                part(K::EarsPointed, Primary, 3),
                part(K::SnoutBroad, Primary, 6),
                part(K::HornsDoubleNose, Feature, 6),
                part(K::TailTufted, Primary, 2),
                part(K::FeetPads, Primary, 5),
            ],
            markings: vec![],
            face_template: 1,
            face: gentle,
        },
        Animal {
            id: "animal.giraffe",
            name: "Giraffe",
            plan: P::TallQuadruped,
            shape: [5, 5, 5, 6, 6, 5, 5, 5],
            treatment: T::Fur,
            colors: [
                [0xf4, 0xcc, 0x84],
                [0xa8, 0x64, 0x34],
                [0xfa, 0xee, 0xd4],
                BLUSH,
                [0x5c, 0x40, 0x34],
                EYES,
            ],
            underside_reach: 2,
            parts: vec![
                part(K::EarsPointed, Primary, 4),
                part(K::SnoutMuzzle, Underside, 4),
                part(K::HornsOssicones, Primary, 5),
                part(K::ManeCrest, Secondary, 4),
                part(K::TailTufted, Primary, 5),
                part(K::FeetHooves, Feature, 5),
            ],
            markings: vec![mark(M::Patches, [0xc0, 0x74, 0x3c], 7, 4)],
            face_template: 1,
            face: gentle,
        },
        Animal {
            id: "animal.snow_leopard",
            name: "Snow leopard",
            plan: P::CompactQuadruped,
            shape: [6, 6, 5, 4, 5, 5, 5, 9],
            treatment: T::Fur,
            colors: [
                [0xe6, 0xe4, 0xdc],
                [0x6c, 0x6c, 0x74],
                [0xfa, 0xfa, 0xf4],
                BLUSH,
                [0xf4, 0xec, 0xdc],
                [0x3c, 0x5c, 0x6c],
            ],
            underside_reach: 5,
            parts: vec![
                part(K::EarsRound, Primary, 4),
                part(K::SnoutMuzzle, Underside, 4),
                part(K::TailFluffy, Primary, 8),
                part(K::FeetBigPaws, Primary, 5),
            ],
            markings: vec![mark(M::Rosettes, [0x6c, 0x6c, 0x74], 7, 3)],
            face_template: 2,
            face: cat,
        },
        Animal {
            id: "animal.sea_turtle",
            name: "Sea turtle",
            plan: P::Crawler,
            shape: [5, 5, 5, 5, 5, 5, 5, 4],
            treatment: T::Smooth,
            colors: [
                [0x9c, 0xb4, 0x78],
                [0x8c, 0x6c, 0x44],
                [0xe8, 0xe0, 0xb8],
                BLUSH,
                [0xd8, 0xc8, 0x94],
                EYES,
            ],
            underside_reach: 3,
            parts: vec![
                part(K::TailStub, Primary, 3),
                part(K::FinsFlippers, Primary, 6),
                part(K::ShellDome, Secondary, 5),
            ],
            markings: vec![mark(M::Mottle, [0x7c, 0x98, 0x5c], 4, 2)],
            face_template: 1,
            face: sea,
        },
        Animal {
            id: "animal.bald_eagle",
            name: "Bald eagle",
            plan: P::Percher,
            shape: [6, 5, 5, 5, 4, 5, 5, 5],
            treatment: T::Feathers,
            colors: [
                [0x6c, 0x4c, 0x3a],
                [0x4c, 0x34, 0x28],
                [0xfa, 0xf8, 0xf2],
                BLUSH,
                [0xf6, 0xc0, 0x40],
                EYES,
            ],
            underside_reach: 0,
            parts: vec![
                part(K::SnoutHookedBeak, Feature, 5),
                part(K::TailFan, Underside, 5),
                part(K::FeetTalons, Feature, 5),
                part(K::WingsFolded, Primary, 5),
            ],
            markings: vec![mark(M::Cap, [0xfa, 0xf8, 0xf2], 10, 5)],
            face_template: 1,
            face: face(EyeShape::Round, 2, 5, MouthStyle::Tiny, CheekStyle::None),
        },
        Animal {
            id: "animal.koala",
            name: "Koala",
            plan: P::Upright,
            shape: [7, 5, 5, 4, 3, 4, 6, 0],
            treatment: T::Fur,
            colors: [
                [0xa8, 0xa8, 0xb0],
                [0x80, 0x80, 0x8a],
                [0xf4, 0xf0, 0xec],
                [0xf4, 0xf0, 0xec],
                [0x4c, 0x48, 0x50],
                EYES,
            ],
            underside_reach: 5,
            parts: vec![
                part(K::EarsRound, Primary, 10),
                part(K::SnoutButton, Feature, 9),
                part(K::FeetHands, Primary, 5),
            ],
            markings: vec![],
            face_template: 1,
            face: bear,
        },
    ]
    .into_iter()
    .map(Animal::preset)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_twenty_animals_each_named_once_and_each_a_valid_design() {
        let animals = animals();
        assert_eq!(animals.len(), 20);
        let mut ids = std::collections::HashSet::new();
        for preset in all() {
            assert!(ids.insert(preset.id), "{} twice", preset.id);
            assert_eq!(preset.design.validate(), Ok(()), "{}", preset.id);
        }
    }

    #[test]
    fn the_animals_span_every_body_plan() {
        let animals = animals();
        for plan in Plan::ALL {
            assert!(
                animals.iter().any(|preset| preset
                    .design
                    .form
                    .sculpt()
                    .is_some_and(|s| s.plan == plan)),
                "{plan:?}"
            );
        }
    }
}
