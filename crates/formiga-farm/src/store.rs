//! What Farm keeps of its own, in its own data folder: drafts, personal presets, where the
//! window was, and a lock that says a window is open. None of it is the colony's: deleting a
//! draft or a preset never touches a creature, and nothing here is read by Desktop.
//!
//! Every file is written whole and atomically, read with a size limit, and checked as it is read.
//! A file that does not check out is set aside under another name, never written over, and Farm
//! opens without it.

use crate::session::now;
use formiga_forms::Design;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use time::OffsetDateTime;

/// Farm's data folder: `FORMIGA_FARM_DATA_DIR` if it is set, for development and tests.
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("FORMIGA_FARM_DATA_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("com", "Formiga", "Formiga Farm")
        .map(|dirs| dirs.data_dir().to_owned())
}

const LOCK: &str = "open.lock";
const DRAFTS: &str = "drafts";
const PRESETS: &str = "presets";
const WINDOW: &str = "window.json";

/// The most drafts and personal presets kept, each: plenty for anyone, and a bound on what is
/// read at start.
pub const MAX_DRAFTS: usize = 64;
pub const MAX_PRESETS: usize = 64;

/// The largest a draft or preset file may be.
const MAX_FILE_BYTES: u64 = 64 * 1024;

/// A name the owner gives a draft or a preset.
pub const MAX_NAME_CHARS: usize = formiga_farm_contract::limits::MAX_DRAFT_NAME_CHARS;

const DRAFT_FORMAT: &str = "formiga.farm.draft";
const PRESET_FORMAT: &str = "formiga.farm.preset";
const VERSION: u32 = 1;

/// Held for as long as a Farm window is open. The lock is the operating system's, so it goes with
/// the window however that closes.
pub struct Open {
    _lock: Option<File>,
}

/// Another Farm window is open.
#[derive(Debug, PartialEq, Eq)]
pub struct Busy;

/// Takes the lock in `data`, or says another Farm has it. Anything else that goes wrong is not
/// another Farm, and nothing is refused for it.
pub fn take(data: &Path) -> Result<Open, Busy> {
    let opened = std::fs::create_dir_all(data).and_then(|()| {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(data.join(LOCK))
    });
    let Ok(file) = opened else {
        return Ok(Open { _lock: None });
    };
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => Err(Busy),
        Ok(()) | Err(TryLockError::Error(_)) => Ok(Open { _lock: Some(file) }),
    }
}

/// A design being worked on, kept whole after every change so a crash loses nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub format: String,
    pub version: u32,
    /// Its file's name, without the extension: 16 lowercase hex digits.
    pub id: String,
    pub name: String,
    pub design: Design,
    /// The preset it began as, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// The creature it is a new look for, by name, if it began as one: for the shelf's label
    /// only. A draft carries nothing else about anyone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at_utc: OffsetDateTime,
}

/// A design the owner keeps to start from again, by a name they gave it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersonalPreset {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub name: String,
    pub design: Design,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at_utc: OffsetDateTime,
}

/// A fresh identifier for a draft or preset.
pub fn new_id() -> String {
    // A session id is 16 random bytes from the operating system, as 32 lowercase hex digits;
    // half of one is plenty here.
    if let Ok(id) = formiga_travel::SessionId::generate() {
        return id.as_str()[..16].to_owned();
    }
    let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
    (nanos as u64)
        .to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_id(text: &str) -> bool {
    text.len() == 16 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A name made safe and short enough to keep.
pub fn clean_name(name: &str, fallback: &str) -> String {
    let name = formiga_travel::sanitize_text(name, MAX_NAME_CHARS);
    if name.is_empty() {
        fallback.to_owned()
    } else {
        name
    }
}

impl Draft {
    pub fn new(
        name: &str,
        design: Design,
        preset: Option<String>,
        for_name: Option<String>,
    ) -> Self {
        Self {
            format: DRAFT_FORMAT.to_owned(),
            version: VERSION,
            id: new_id(),
            name: clean_name(name, "Untitled"),
            design,
            preset,
            for_name,
            saved_at_utc: now(),
        }
    }

    fn valid(&self) -> bool {
        self.format == DRAFT_FORMAT
            && self.version >= 1
            && is_id(&self.id)
            && formiga_travel::is_sanitized(&self.name, MAX_NAME_CHARS)
            && !self.name.is_empty()
            && self.design.validate().is_ok()
            && self
                .for_name
                .as_deref()
                .is_none_or(|name| formiga_travel::is_sanitized(name, MAX_NAME_CHARS))
    }
}

impl PersonalPreset {
    pub fn new(name: &str, design: Design) -> Self {
        Self {
            format: PRESET_FORMAT.to_owned(),
            version: VERSION,
            id: new_id(),
            name: clean_name(name, "My Formiga"),
            design,
            saved_at_utc: now(),
        }
    }

    fn valid(&self) -> bool {
        self.format == PRESET_FORMAT
            && self.version >= 1
            && is_id(&self.id)
            && formiga_travel::is_sanitized(&self.name, MAX_NAME_CHARS)
            && !self.name.is_empty()
            && self.design.validate().is_ok()
    }
}

/// Farm's own shelves, kept in its data folder, or nowhere if it has none.
pub struct Store {
    data: Option<PathBuf>,
}

impl Store {
    pub fn new(data: Option<PathBuf>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> Option<&Path> {
        self.data.as_deref()
    }

    fn dir(&self, name: &str) -> Option<PathBuf> {
        self.data.as_ref().map(|data| data.join(name))
    }

    /// The newest drafts that read, newest first, as many as the shelf holds. One that does not
    /// read is set aside.
    pub fn drafts(&self) -> Vec<Draft> {
        let mut drafts = self.every_draft();
        drafts.truncate(MAX_DRAFTS);
        drafts
    }

    /// Every draft that reads, newest first, with any past the shelf's bound still among them,
    /// so that [`Self::save_draft`] can see which to clear away.
    fn every_draft(&self) -> Vec<Draft> {
        let mut drafts: Vec<Draft> = read_all(self.dir(DRAFTS), MAX_DRAFTS * 2, Draft::valid);
        drafts.sort_by(|a, b| b.saved_at_utc.cmp(&a.saved_at_utc).then(a.id.cmp(&b.id)));
        drafts
    }

    pub fn presets(&self) -> Vec<PersonalPreset> {
        let mut presets: Vec<PersonalPreset> =
            read_all(self.dir(PRESETS), MAX_PRESETS * 2, PersonalPreset::valid);
        presets.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.id.cmp(&b.id))
        });
        presets.truncate(MAX_PRESETS);
        presets
    }

    /// Keep `draft`, whole, stamped with the time.
    pub fn save_draft(&self, draft: &mut Draft) -> std::io::Result<()> {
        draft.saved_at_utc = now();
        write(self.dir(DRAFTS), &draft.id, draft)?;
        // Keep the shelf to its bound: the oldest go first, and never the one just kept, whatever
        // the clock said when the others were.
        for old in self.every_draft().iter().skip(MAX_DRAFTS) {
            if old.id != draft.id {
                self.delete_draft(&old.id);
            }
        }
        Ok(())
    }

    pub fn delete_draft(&self, id: &str) {
        delete(self.dir(DRAFTS), id);
    }

    pub fn save_preset(&self, preset: &PersonalPreset) -> std::io::Result<()> {
        if self.presets().len() >= MAX_PRESETS {
            return Err(std::io::Error::other("the shelf of presets is full"));
        }
        write(self.dir(PRESETS), &preset.id, preset)
    }

    pub fn delete_preset(&self, id: &str) {
        delete(self.dir(PRESETS), id);
    }
}

fn write<T: Serialize>(dir: Option<PathBuf>, id: &str, value: &T) -> std::io::Result<()> {
    let Some(dir) = dir else {
        return Ok(());
    };
    if !is_id(id) {
        return Err(std::io::Error::other("not a draft's name"));
    }
    std::fs::create_dir_all(&dir)?;
    let mut bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(std::io::Error::other("too large to keep"));
    }
    formiga_travel::write_atomically(&dir.join(format!("{id}.json")), &bytes)
}

/// Remove `id`'s file from `dir`. Only an id is ever taken as a file's name, so nothing outside
/// `dir` can be named.
fn delete(dir: Option<PathBuf>, id: &str) {
    if let Some(dir) = dir
        && is_id(id)
    {
        let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
    }
}

/// What reads of the `most` files in `dir` written last. Those are read rather than the first by
/// name, so the file just written is always among them, however many others there are.
fn read_all<T: for<'de> Deserialize<'de>>(
    dir: Option<PathBuf>,
    most: usize,
    valid: fn(&T) -> bool,
) -> Vec<T> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut paths: Vec<(Option<SystemTime>, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let written = entry.metadata().and_then(|meta| meta.modified()).ok();
            (written, entry.path())
        })
        .filter(|(_, path)| {
            path.extension().is_some_and(|ext| ext == "json")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(is_id)
        })
        .collect();
    paths.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut kept = Vec::new();
    for (_, path) in paths.into_iter().take(most) {
        let read = formiga_farm_contract::read_bounded(&path, MAX_FILE_BYTES)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<T>(&bytes).ok())
            .filter(valid);
        match read {
            Some(value) => kept.push(value),
            None => set_aside(&path),
        }
    }
    kept
}

/// A file that cannot be read is moved aside, never deleted and never written over, so that
/// nothing the owner made is lost to a bug, and Farm opens without it.
fn set_aside(path: &Path) {
    let aside = path.with_extension(format!(
        "unreadable-{}",
        OffsetDateTime::now_utc().unix_timestamp()
    ));
    eprintln!("formiga-farm: setting aside {}", path.display());
    let _ = std::fs::rename(path, aside);
}

/// Where the window was and how big, in logical points.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPlace {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl WindowPlace {
    pub fn load(data: &Path) -> Option<Self> {
        let bytes = formiga_farm_contract::read_bounded(&data.join(WINDOW), 4096).ok()?;
        let place: Self = serde_json::from_slice(&bytes).ok()?;
        let sane = [place.x, place.y, place.width, place.height]
            .iter()
            .all(|value| value.is_finite())
            && (480.0..=8000.0).contains(&place.width)
            && (360.0..=8000.0).contains(&place.height);
        sane.then_some(place)
    }

    pub fn save(&self, data: &Path) {
        let written = std::fs::create_dir_all(data).and_then(|()| {
            let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
            formiga_travel::write_atomically(&data.join(WINDOW), &bytes)
        });
        if let Err(error) = written {
            eprintln!("formiga-farm: could not remember where the window was: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("formiga-farm-store-{name}-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_draft_is_kept_whole_and_read_back_exactly() {
        let dir = scratch("drafts");
        let store = Store::new(Some(dir.clone()));
        let mut draft = Draft::new("Pip", crate::presets::blank(), Some("blank".into()), None);
        store.save_draft(&mut draft).unwrap();
        assert_eq!(store.drafts(), vec![draft.clone()]);
        store.delete_draft(&draft.id);
        assert!(store.drafts().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_draft_that_does_not_read_is_set_aside_and_farm_opens_without_it() {
        let dir = scratch("corrupt");
        let store = Store::new(Some(dir.clone()));
        let mut good = Draft::new("Good", crate::presets::blank(), None, None);
        store.save_draft(&mut good).unwrap();
        let bad = dir.join(DRAFTS).join("0123456789abcdef.json");
        std::fs::write(&bad, b"{ this is not a draft").unwrap();
        let mut odd = Draft::new("Odd", crate::presets::blank(), None, None);
        if let formiga_forms::Form::Sculpted { sculpt } = &mut odd.design.form {
            sculpt.shape.head = 200;
        }
        let odd_path = dir.join(DRAFTS).join(format!("{}.json", odd.id));
        std::fs::write(&odd_path, serde_json::to_vec(&odd).unwrap()).unwrap();
        let drafts = store.drafts();
        assert_eq!(drafts, vec![good]);
        assert!(!bad.exists() && !odd_path.exists(), "both set aside");
        let aside = std::fs::read_dir(dir.join(DRAFTS)).unwrap().count();
        assert_eq!(aside, 3, "kept, not deleted");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_full_shelf_of_drafts_lets_the_oldest_go_and_always_shows_the_one_just_kept() {
        let dir = scratch("full");
        let store = Store::new(Some(dir.clone()));
        // A full shelf, kept an hour ago, oldest last, by ids that all sort before the new one's.
        let an_hour_ago = now() - time::Duration::hours(1);
        for index in 0..MAX_DRAFTS {
            let mut old = Draft::new("Old", crate::presets::blank(), None, None);
            old.id = format!("{index:016x}");
            old.saved_at_utc = an_hour_ago - time::Duration::seconds(index as i64);
            write(store.dir(DRAFTS), &old.id, &old).unwrap();
        }
        let mut draft = Draft::new("New", crate::presets::blank(), None, None);
        draft.id = "ffffffffffffffff".into();
        store.save_draft(&mut draft).unwrap();
        let drafts = store.drafts();
        assert_eq!(drafts.len(), MAX_DRAFTS);
        assert_eq!(drafts[0], draft, "the one just kept is on the shelf, first");
        let oldest = format!("{:016x}", MAX_DRAFTS - 1);
        assert!(drafts.iter().all(|kept| kept.id != oldest));
        let files = std::fs::read_dir(dir.join(DRAFTS)).unwrap().count();
        assert_eq!(files, MAX_DRAFTS, "the oldest is cleared away");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_draft_just_kept_is_on_the_shelf_however_many_older_files_there_are() {
        let dir = scratch("backlog");
        let store = Store::new(Some(dir.clone()));
        // More files than are ever read at once, all written well before, by ids that sort
        // before the new one's.
        let an_hour_ago = SystemTime::now() - std::time::Duration::from_secs(3600);
        for index in 0..MAX_DRAFTS * 3 {
            let mut old = Draft::new("Old", crate::presets::blank(), None, None);
            old.id = format!("{index:016x}");
            old.saved_at_utc = now() - time::Duration::hours(1);
            write(store.dir(DRAFTS), &old.id, &old).unwrap();
            let path = dir.join(DRAFTS).join(format!("{}.json", old.id));
            File::options()
                .write(true)
                .open(path)
                .and_then(|file| file.set_modified(an_hour_ago))
                .unwrap();
        }
        let mut draft = Draft::new("New", crate::presets::blank(), None, None);
        draft.id = "ffffffffffffffff".into();
        store.save_draft(&mut draft).unwrap();
        assert_eq!(store.drafts()[0], draft);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_a_preset_by_a_name_that_is_not_an_id_touches_nothing() {
        let dir = scratch("paths");
        let store = Store::new(Some(dir.clone()));
        let preset = PersonalPreset::new("Mine", crate::presets::blank());
        store.save_preset(&preset).unwrap();
        store.delete_preset("../presets/whatever");
        store.delete_preset("");
        assert_eq!(store.presets().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
