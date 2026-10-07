//! A creature's form: the semantic design Formiga Farm edits, and how it is drawn.
//!
//! A [`Design`] is everything about how a creature looks and nothing about who it is. Its
//! [`Form`] is one of three:
//!
//! - **Original**: the look of a companion made before recipes, drawn from its genes;
//! - **Companion**: one of Desktop's five companion plans, exactly as its recipe
//!   ([`formiga_core::CreatureDesign`]) says;
//! - **Sculpted**: a [`Sculpt`] on one of the newer [`Plan`]s, quadrupeds large and small, an
//!   upright body, a floater, a crawler and a percher, with parts fitted to named slots, a coat
//!   and its markings.
//!
//! The first two are drawn by Desktop's own renderer and nothing here changes how; a creature
//! that keeps its body keeps every pixel. A sculpted form is drawn by [`DesignRenderer`] in the
//! same 48-pixel frame, wearing the same faces, and answering the same clips, so it can stand in
//! for a companion anywhere one is drawn; and it can be drawn in high definition, the same frame
//! with [`DETAIL`] pixels to each of its own, finer pixel art in the same style.
//!
//! Every value in a design is small, bounded and named for what it does. A design read from
//! anywhere else is checked with [`Design::validate`] and refused rather than repaired, and
//! [`Design::revision`] names it exactly, so a change proposed against one revision can be told
//! apart from a creature that has changed since.
//!
//! This crate is a draft kept beside Formiga Farm. The model belongs in Desktop's `formiga-core`
//! and the drawing in its `formiga-art`, and both move there when Desktop adopts them.

mod design;
mod render;
mod sculpt;

pub use design::{Design, Form, bounded_face, plain_face};
pub use render::{Anchors, DesignRenderer, Intent};
pub use sculpt::{
    Coat, Dimension, Ink, Locomotion, MAX_MARKINGS, MIDDLE, Marking, MarkingKind, NUDGE, Part,
    PartKind, Plan, STEPS, Sculpt, Shape, Slot, Treatment,
};

/// The version of the design model this build reads and writes: which plans, parts, markings
/// and treatments exist, and what each value means. A design from a newer model may name
/// something this build does not know, and is refused rather than guessed at.
pub const DESIGN_VERSION: u32 = 1;

/// How many pixels across each of a frame's 48 a sculpted form is drawn with in high
/// definition ([`DesignRenderer::frame_hd`]): what Farm shows on its stage, and what Desktop
/// draws a reshaped companion with, at the same size on screen as every other.
pub const DETAIL: u32 = 2;

/// Why a design cannot be used.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FormError {
    #[error("the design is not usable: {0}")]
    Invalid(String),
}
