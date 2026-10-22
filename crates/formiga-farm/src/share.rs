//! Design codes: a look written as one line of text, to send to a friend or keep in a note and
//! paste back into Farm later.
//!
//! A code carries a design and the name it was given, nothing else: no creature, no colony and
//! no lineage. It is read as untrusted text from anywhere: bounded in length, versioned, and
//! refused unless its design is exactly as [`Design::validate`] would leave it, never repaired.
//! A look that comes from a creature's own genes (`Form::Original`) belongs to that creature, so
//! it has no code.

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use formiga_core::forms::{Design, Form};
use serde::{Deserialize, Serialize};

/// What every code begins with, naming its version, so a code from a later Farm is told apart
/// from text that is not a code at all.
pub const PREFIX: &str = "formiga-look-1.";

/// The longest code Farm reads. The largest design Farm makes is well under half of it.
pub const MAX_CODE_CHARS: usize = 8 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    name: String,
    design: Design,
}

/// A look read from a code.
#[derive(Debug, PartialEq)]
pub struct Shared {
    pub name: String,
    pub design: Design,
}

/// Whether `design` can be written as a code at all.
pub fn shareable(design: &Design) -> bool {
    !matches!(design.form, Form::Original)
}

/// `design`, called `name`, as a code.
pub fn encode(design: &Design, name: &str) -> Result<String> {
    if !shareable(design) {
        bail!("This look comes from the Formiga's own genes, so it has no code.");
    }
    let payload = Payload {
        name: crate::store::clean_name(name, "Shared look"),
        design: design.normalized(),
    };
    let json = serde_json::to_vec(&payload).context("The look could not be written.")?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json)))
}

/// The look in `text`, which may have spaces or line breaks around or inside it from being
/// pasted.
pub fn decode(text: &str) -> Result<Shared> {
    // One character past the limit is enough to know it is too long, so a huge paste is never
    // copied whole.
    let code: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .take(MAX_CODE_CHARS + 1)
        .collect();
    if code.len() > MAX_CODE_CHARS {
        bail!("That is too long to be a design code.");
    }
    let Some(body) = code.strip_prefix(PREFIX) else {
        if code.starts_with("formiga-look-") {
            bail!("That code is from a newer Formiga Farm.");
        }
        bail!("That is not a design code.");
    };
    let json = URL_SAFE_NO_PAD
        .decode(body)
        .ok()
        .context("That design code is damaged.")?;
    let payload: Payload = serde_json::from_slice(&json)
        .ok()
        .context("That design code is damaged.")?;
    if !shareable(&payload.design) {
        bail!("That design code has no look in it.");
    }
    payload
        .design
        .validate()
        .ok()
        .context("That design code asks for something Farm cannot draw.")?;
    if !formiga_travel::is_sanitized(&payload.name, crate::store::MAX_NAME_CHARS) {
        bail!("That design code's name is not one Farm can show.");
    }
    Ok(Shared {
        name: payload.name,
        design: payload.design,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_preset() -> Vec<crate::presets::Preset> {
        crate::presets::all()
    }

    #[test]
    fn every_preset_comes_back_from_its_code_exactly() {
        for preset in every_preset() {
            let code = encode(&preset.design, preset.name).unwrap();
            assert!(
                code.len() < MAX_CODE_CHARS / 2,
                "{} is {} long",
                preset.id,
                code.len()
            );
            let shared = decode(&code).unwrap();
            assert_eq!(shared.design, preset.design.normalized(), "{}", preset.id);
            assert_eq!(shared.name, preset.name);
        }
    }

    #[test]
    fn a_code_pasted_with_spaces_and_line_breaks_still_reads() {
        let preset = &every_preset()[0];
        let code = encode(&preset.design, "Mine").unwrap();
        let (a, b) = code.split_at(code.len() / 2);
        let pasted = format!("  {a}\n  {b}\n");
        assert_eq!(decode(&pasted).unwrap().design, preset.design.normalized());
    }

    #[test]
    fn a_look_from_genes_has_no_code() {
        let mut design = every_preset()[0].design.clone();
        design.form = Form::Original;
        assert!(!shareable(&design));
        assert!(encode(&design, "Mine").is_err());
    }

    #[test]
    fn anything_that_is_not_a_good_code_is_refused_and_never_repaired() {
        assert!(decode("").is_err());
        assert!(decode("hello").is_err());
        assert!(decode("formiga-look-1.***").is_err());
        assert!(decode(&format!("{PREFIX}{}", "A".repeat(MAX_CODE_CHARS))).is_err());
        let newer = decode("formiga-look-2.abc").unwrap_err().to_string();
        assert!(newer.contains("newer"), "{newer}");

        // A design out of range is refused, though Farm could clamp it.
        let preset = every_preset()
            .into_iter()
            .find(|preset| matches!(preset.design.form, Form::Sculpted { .. }))
            .unwrap();
        let mut json = serde_json::to_value(Payload {
            name: "Mine".into(),
            design: preset.design.clone(),
        })
        .unwrap();
        json["design"]["form"]["sculpt"]["parts"][0]["tilt"] = 120.into();
        let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json.to_string()));
        assert!(decode(&code).is_err());

        // So is anything a code was never meant to carry.
        let mut json = serde_json::to_value(Payload {
            name: "Mine".into(),
            design: preset.design,
        })
        .unwrap();
        json["creature"] = 7.into();
        let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json.to_string()));
        assert!(decode(&code).is_err());
    }
}
