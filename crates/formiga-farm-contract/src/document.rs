//! Reading and writing Farm documents: bounded before parsing, version-checked before shaping,
//! validated both ways, and written whole or not at all. How is the rulebook's, the same for a
//! trip's documents and a visit's; what each document must keep, and the words for what went
//! wrong, are Farm's, under Farm's own version.

use crate::FARM_FORMAT_VERSION;
use formiga_expansion_rulebook::{self as rulebook, Kind, RulebookError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io;
use std::path::Path;

pub use formiga_expansion_rulebook::sha256_hex;
pub(crate) use formiga_expansion_rulebook::{header_ok, is_lower_hex};

#[derive(Debug, thiserror::Error)]
pub enum FarmError {
    #[error("the Farm file could not be read or written: {0}")]
    Io(#[from] io::Error),
    #[error("the Farm file is larger than any Farm file can be ({limit} bytes)")]
    TooLarge { limit: u64 },
    #[error("the Farm file could not be read: {0}")]
    Json(#[from] serde_json::Error),
    #[error("expected a {expected} file, found {found:?}")]
    WrongFormat {
        expected: &'static str,
        found: String,
    },
    #[error(
        "the Farm file needs a reader of Farm version {needs}, and this build reads up to \
         version {reads}"
    )]
    UnsupportedVersion { needs: u32, reads: u32 },
    #[error("the Farm file is not usable: {0}")]
    Invalid(String),
}

impl FarmError {
    pub(crate) fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }
}

impl From<RulebookError> for FarmError {
    fn from(error: RulebookError) -> Self {
        match error {
            RulebookError::Io(error) => Self::Io(error),
            RulebookError::TooLarge { limit } => Self::TooLarge { limit },
            RulebookError::Json(error) => Self::Json(error),
            RulebookError::WrongFormat { expected, found } => Self::WrongFormat { expected, found },
            RulebookError::UnsupportedVersion { needs, reads } => {
                Self::UnsupportedVersion { needs, reads }
            }
            RulebookError::Invalid(reason) => Self::Invalid(reason),
        }
    }
}

/// One kind of Farm document.
pub trait FarmDocument: Serialize + DeserializeOwned {
    /// Its `format` field.
    const FORMAT: &'static str;
    /// The largest it may be, checked before it is parsed and before it is written.
    const MAX_BYTES: u64;
    /// Every bound and reference it must keep, checked both when it is written and when it is
    /// read.
    fn validate(&self) -> Result<(), FarmError>;
}

/// How the rulebook holds a document of kind `T`: under its own format and bounds, read as Farm
/// version [`FARM_FORMAT_VERSION`].
fn kind<T: FarmDocument>() -> Kind {
    Kind {
        format: T::FORMAT,
        max_bytes: T::MAX_BYTES,
        reads: FARM_FORMAT_VERSION,
    }
}

/// Read a document from its bytes.
pub fn decode<T: FarmDocument>(bytes: &[u8]) -> Result<T, FarmError> {
    let document: T = rulebook::decode(bytes, kind::<T>())?;
    document.validate()?;
    Ok(document)
}

/// A document's bytes, once it has been validated. The same document always gives the same bytes.
pub fn encode<T: FarmDocument>(document: &T) -> Result<Vec<u8>, FarmError> {
    document.validate()?;
    Ok(rulebook::encode(document, kind::<T>())?)
}

/// Read a document from a file, refusing one larger than the document can be without reading the
/// rest of it. Returns the document and the bytes it was read from, which is what a seal is made
/// of.
pub fn read_document<T: FarmDocument>(path: &Path) -> Result<(T, Vec<u8>), FarmError> {
    let bytes = read_bounded(path, T::MAX_BYTES)?;
    Ok((decode(&bytes)?, bytes))
}

/// Validate a document and write it whole: to a temporary file beside it, synced, then renamed
/// into place. Returns the bytes written.
pub fn write_document<T: FarmDocument>(path: &Path, document: &T) -> Result<Vec<u8>, FarmError> {
    let bytes = encode(document)?;
    rulebook::write_atomically(path, &bytes)?;
    Ok(bytes)
}

/// At most `limit` bytes of a file, or [`FarmError::TooLarge`] if there are more.
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, FarmError> {
    Ok(rulebook::read_bounded(path, limit)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FarmSnapshot;

    #[test]
    fn a_snapshot_from_a_newer_reader_is_refused_in_farm_words() {
        let newer = FARM_FORMAT_VERSION + 1;
        let bytes = serde_json::to_vec(&serde_json::json!({
            "format": crate::SNAPSHOT_FORMAT,
            "version": newer + 4,
            "min_reader_version": newer,
            "anything": ["shaped", "differently"],
        }))
        .unwrap();
        let error = decode::<FarmSnapshot>(&bytes).unwrap_err();
        assert!(
            matches!(
                error,
                FarmError::UnsupportedVersion { needs, reads }
                    if (needs, reads) == (newer, FARM_FORMAT_VERSION)
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("Farm version"), "{error}");
    }

    #[test]
    fn a_document_of_another_kind_is_refused_for_what_it_is() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "format": "formiga.home.snapshot",
            "version": 1,
        }))
        .unwrap();
        assert!(matches!(
            decode::<FarmSnapshot>(&bytes),
            Err(FarmError::WrongFormat { .. })
        ));
    }

    #[test]
    fn a_file_larger_than_its_limit_is_refused_in_farm_words() {
        let directory =
            std::env::temp_dir().join(format!("formiga-farm-bounded-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("big.json");
        std::fs::write(&path, vec![b' '; 2048]).unwrap();
        let error = read_bounded(&path, 1024).unwrap_err();
        assert!(
            matches!(error, FarmError::TooLarge { limit: 1024 }),
            "{error:?}"
        );
        assert!(error.to_string().contains("Farm file"), "{error}");
        assert_eq!(read_bounded(&path, 2048).unwrap().len(), 2048);
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
