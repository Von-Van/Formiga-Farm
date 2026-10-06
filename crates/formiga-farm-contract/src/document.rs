//! Reading and writing Farm documents: bounded before parsing, version-checked before shaping,
//! validated both ways, and written whole or not at all. The same discipline as a trip's
//! documents and a visit's, under Farm's own version.

use crate::FARM_FORMAT_VERSION;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub use formiga_travel::sha256_hex;

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

    /// Whether the file was simply not there.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Io(error) if error.kind() == io::ErrorKind::NotFound)
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

/// The three fields every document starts with, checked on the raw JSON before it is shaped, so a
/// document from a newer writer is refused for its version rather than for whatever shape it
/// happens to have.
fn check_header(value: &Value, expected: &'static str) -> Result<(), FarmError> {
    let format = value
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if format != expected {
        return Err(FarmError::WrongFormat {
            expected,
            found: format.chars().take(64).collect(),
        });
    }
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .filter(|version| *version >= 1)
        .ok_or_else(|| FarmError::invalid("the file names no version"))?;
    let needs = value
        .get("min_reader_version")
        .map(|needs| {
            needs
                .as_u64()
                .ok_or_else(|| FarmError::invalid("min_reader_version is not a number"))
        })
        .transpose()?
        .unwrap_or(version);
    if needs > version {
        return Err(FarmError::invalid(
            "min_reader_version is newer than the version it was written as",
        ));
    }
    if needs > u64::from(FARM_FORMAT_VERSION) {
        return Err(FarmError::UnsupportedVersion {
            needs: u32::try_from(needs).unwrap_or(u32::MAX),
            reads: FARM_FORMAT_VERSION,
        });
    }
    Ok(())
}

/// The three header fields every document is written with: what it is, the version this build
/// writes, and the oldest reader that may use it.
pub(crate) fn header_ok(format: &str, version: u32, min_reader: u32, expected: &str) -> bool {
    format == expected && version >= 1 && (1..=version).contains(&min_reader)
}

/// Read a document from its bytes.
pub fn decode<T: FarmDocument>(bytes: &[u8]) -> Result<T, FarmError> {
    if bytes.len() as u64 > T::MAX_BYTES {
        return Err(FarmError::TooLarge {
            limit: T::MAX_BYTES,
        });
    }
    let value: Value = serde_json::from_slice(bytes)?;
    check_header(&value, T::FORMAT)?;
    let document: T = serde_json::from_value(value)?;
    document.validate()?;
    Ok(document)
}

/// A document's bytes, once it has been validated. The same document always gives the same bytes.
pub fn encode<T: FarmDocument>(document: &T) -> Result<Vec<u8>, FarmError> {
    document.validate()?;
    let mut bytes = serde_json::to_vec_pretty(document)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > T::MAX_BYTES {
        return Err(FarmError::TooLarge {
            limit: T::MAX_BYTES,
        });
    }
    Ok(bytes)
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
    formiga_travel::write_atomically(path, &bytes)?;
    Ok(bytes)
}

/// At most `limit` bytes of a file, or [`FarmError::TooLarge`] if there are more.
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, FarmError> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(FarmError::TooLarge { limit });
    }
    Ok(bytes)
}

/// Whether `text` is `digits` lowercase hex digits.
pub(crate) fn is_lower_hex(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_from_a_newer_reader_is_refused_for_its_version() {
        let newer = FARM_FORMAT_VERSION + 1;
        let value = serde_json::json!({
            "format": "formiga.farm.snapshot",
            "version": newer + 4,
            "min_reader_version": newer,
            "anything": ["shaped", "differently"],
        });
        match check_header(&value, "formiga.farm.snapshot") {
            Err(FarmError::UnsupportedVersion { needs, reads }) => {
                assert_eq!((needs, reads), (newer, FARM_FORMAT_VERSION))
            }
            other => panic!("expected a version refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_newer_writer_that_older_readers_may_use_is_accepted() {
        let value = serde_json::json!({
            "format": "formiga.farm.proposal",
            "version": 4,
            "min_reader_version": 1,
        });
        assert!(check_header(&value, "formiga.farm.proposal").is_ok());
    }

    #[test]
    fn a_document_of_another_kind_is_refused_for_what_it_is() {
        let value = serde_json::json!({ "format": "formiga.home.snapshot", "version": 1 });
        assert!(matches!(
            check_header(&value, "formiga.farm.snapshot"),
            Err(FarmError::WrongFormat { .. })
        ));
    }
}
