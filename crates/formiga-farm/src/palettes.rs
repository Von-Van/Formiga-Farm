//! Colours to start from: a design's own colours taken as a palette, put on another design, and
//! offered as swatches beside every colour. A palette is only colours. It carries nothing about a
//! body or anyone in the colony, and putting one on changes colours and nothing else.

use formiga_core::forms::{Design, Form, MAX_MARKINGS};

/// The most colours a palette holds: a sculpted coat's five and its eyes, and one for each
/// marking.
pub const MAX_COLORS: usize = 6 + MAX_MARKINGS;

/// The colours of `design`, in the order a palette keeps them: for a sculpted form its primary,
/// secondary, underside, accent, feature and eye colours, then each marking's; for a companion
/// its coat and accent. A look from before recipes has none to give.
pub fn colors_of(design: &Design) -> Vec<[u8; 3]> {
    match &design.form {
        Form::Sculpted { sculpt } => {
            let coat = sculpt.coat;
            [
                coat.primary,
                coat.secondary,
                coat.underside,
                coat.accent,
                coat.feature,
                coat.eyes,
            ]
            .into_iter()
            .chain(sculpt.markings.iter().map(|marking| marking.color))
            .collect()
        }
        Form::Companion { recipe } => vec![recipe.coat, recipe.accent],
        Form::Original => Vec::new(),
    }
}

/// `design` wearing `colors`, taken in the order [`colors_of`] gives them. A companion's palette
/// on a sculpted form colours its primary and secondary (and accent) and leaves the rest; a
/// sculpted form's palette on a companion colours its coat and accent from the primary and
/// secondary. Colours a palette does not have are left as they are, and nothing but colour
/// changes.
pub fn with_colors(design: &Design, colors: &[[u8; 3]]) -> Design {
    let mut design = design.clone();
    let at = |index: usize| colors.get(index).copied();
    match &mut design.form {
        Form::Sculpted { sculpt } => {
            let coat = &mut sculpt.coat;
            if colors.len() == 2 {
                coat.primary = colors[0];
                coat.secondary = colors[1];
                coat.accent = colors[1];
            } else {
                for (index, slot) in [
                    &mut coat.primary,
                    &mut coat.secondary,
                    &mut coat.underside,
                    &mut coat.accent,
                    &mut coat.feature,
                    &mut coat.eyes,
                ]
                .into_iter()
                .enumerate()
                {
                    if let Some(color) = at(index) {
                        *slot = color;
                    }
                }
                for (index, marking) in sculpt.markings.iter_mut().enumerate() {
                    if let Some(color) = at(6 + index) {
                        marking.color = color;
                    }
                }
            }
        }
        Form::Companion { recipe } => {
            if let Some(coat) = at(0) {
                recipe.coat = coat;
            }
            if let Some(accent) = at(1) {
                recipe.accent = accent;
            }
        }
        Form::Original => {}
    }
    design
}

/// Every built-in animal's colours among `presets`, by its name, to start from.
pub fn animals(presets: &[crate::presets::Preset]) -> Vec<(&'static str, Vec<[u8; 3]>)> {
    presets
        .iter()
        .filter(|preset| preset.id.starts_with("animal."))
        .map(|preset| (preset.name, colors_of(&preset.design)))
        .filter(|(_, colors)| !colors.is_empty())
        .collect()
}

/// Swatches to offer beside a colour: the colours already in `design` first, then those of
/// `palettes`, each once, at most `most`.
pub fn swatches<'a>(
    design: &Design,
    palettes: impl IntoIterator<Item = &'a [[u8; 3]]>,
    most: usize,
) -> Vec<[u8; 3]> {
    let mut seen = Vec::new();
    let own = colors_of(design);
    for color in own
        .iter()
        .copied()
        .chain(palettes.into_iter().flatten().copied())
    {
        if !seen.contains(&color) {
            seen.push(color);
        }
        if seen.len() == most {
            break;
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str) -> Design {
        crate::presets::find(id).unwrap().design
    }

    #[test]
    fn a_palette_put_back_on_its_own_design_changes_nothing() {
        for preset in crate::presets::all() {
            let colors = colors_of(&preset.design);
            assert!(colors.len() <= MAX_COLORS, "{}", preset.id);
            assert_eq!(with_colors(&preset.design, &colors), preset.design);
        }
    }

    #[test]
    fn a_palette_changes_colours_and_nothing_else() {
        let fox = preset("animal.red_fox");
        let panda = colors_of(&preset("animal.giant_panda"));
        let recoloured = with_colors(&fox, &panda);
        assert_eq!(recoloured.face, fox.face);
        let (Form::Sculpted { sculpt: before }, Form::Sculpted { sculpt: after }) =
            (&fox.form, &recoloured.form)
        else {
            panic!("both are sculpted")
        };
        assert_eq!(after.plan, before.plan);
        assert_eq!(after.shape, before.shape);
        assert_eq!(after.parts, before.parts);
        assert_eq!(after.coat.treatment, before.coat.treatment);
        assert_eq!(after.markings.len(), before.markings.len());
        assert_eq!(after.coat.primary, panda[0]);
        assert_eq!(after.coat.eyes, panda[5]);
        assert!(recoloured.validate().is_ok());
    }

    #[test]
    fn a_companion_and_a_sculpted_form_take_each_others_main_colours() {
        let round = crate::presets::all()
            .into_iter()
            .find(|preset| matches!(preset.design.form, Form::Companion { .. }))
            .unwrap()
            .design;
        let tiger = preset("animal.tiger");
        let Form::Companion { recipe } = with_colors(&round, &colors_of(&tiger)).form else {
            panic!("still a companion")
        };
        let Form::Sculpted { sculpt } = &tiger.form else {
            panic!("the tiger is sculpted")
        };
        assert_eq!(
            (recipe.coat, recipe.accent),
            (sculpt.coat.primary, sculpt.coat.secondary)
        );
        let Form::Sculpted { sculpt: striped } = with_colors(&tiger, &colors_of(&round)).form
        else {
            panic!("still sculpted")
        };
        let companion = colors_of(&round);
        assert_eq!(striped.coat.primary, companion[0]);
        assert_eq!(striped.coat.secondary, companion[1]);
        assert_eq!(striped.coat.eyes, sculpt.coat.eyes, "the rest is left");
    }

    #[test]
    fn swatches_start_with_the_designs_own_colours_and_never_repeat() {
        let fox = preset("animal.red_fox");
        let panda = colors_of(&preset("animal.giant_panda"));
        let swatches = swatches(&fox, [panda.as_slice()], 12);
        assert_eq!(swatches[0], colors_of(&fox)[0]);
        assert!(swatches.len() <= 12);
        for (index, color) in swatches.iter().enumerate() {
            assert!(!swatches[index + 1..].contains(color));
        }
    }
}
