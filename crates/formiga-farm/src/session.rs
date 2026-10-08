//! Farm's side of a session, as the design contract lays it out, and the rehearsal that stands
//! in for Desktop when Farm is opened on its own.
//!
//! Desktop starts Farm with `--formiga-farm <session directory>`. Farm reads the snapshot there
//! and answers once with an acknowledgement. Each time the owner applies a design, Farm writes it
//! as a proposal, whole, with a serial one higher than the last, and watches for Desktop's
//! verdict on it. A recall, or the snapshot disappearing, ends the session. Desktop never depends
//! on any of it: the creature goes on living on the desktop the whole time, and keeps its look
//! unless Desktop keeps a new one.

use anyhow::{Context, Result, bail};
use formiga_core::{CreatureId, SaveFile};
use formiga_farm_contract::{
    ACK_FILE, Accepted, AckRefusal, Current, FARM_FORMAT_VERSION, FarmAck, FarmDocument, FarmError,
    FarmProposal, FarmRecall, FarmSnapshot, FarmVerdict, Lineage, PROPOSAL_FILE, ProposalKind,
    RECALL_FILE, SNAPSHOT_FILE, SessionId, SessionSeal, VERDICT_FILE, Verdict, accept_proposal,
    apply_design, decode, limits, read_bounded, read_document, sha256_hex, write_document,
};
use formiga_forms::{Design, Sculpt};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub const FARM_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A session opened by Desktop.
pub struct Visit {
    dir: PathBuf,
    seal: SessionSeal,
    serial: u32,
}

/// Desktop, or the rehearsal standing in for it.
pub enum Host {
    Desktop(Visit),
    Rehearsal(Box<Rehearsal>),
}

/// What one look at the session found.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Nothing,
    /// Desktop's verdict on the proposal waiting.
    Verdict(Verdict),
    /// Desktop ended the session.
    Recalled,
}

/// Reads what Desktop left in `dir` and answers it: accepted, with the snapshot ready, or
/// refused, with the reason written for Desktop and returned as an error. `busy` when another
/// Farm window is open, which refuses the session whatever it holds.
pub fn arrive(dir: &Path, busy: bool) -> Result<(Visit, FarmSnapshot)> {
    // The session is named by its directory, and nothing else about the path is trusted.
    let session = dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(SessionId::parse)
        .with_context(|| format!("{} is not a session's directory", dir.display()))?;
    let bytes = read_bounded(&dir.join(SNAPSHOT_FILE), limits::MAX_SNAPSHOT_BYTES)
        .context("could not read the session's snapshot")?;
    let seal = SessionSeal {
        session_id: session.clone(),
        snapshot_sha256: sha256_hex(&bytes),
    };
    let refuse = |refusal: AckRefusal, why: String| -> Result<(Visit, FarmSnapshot)> {
        write_document(
            &dir.join(ACK_FILE),
            &FarmAck::refused(&seal, FARM_VERSION, refusal),
        )?;
        bail!("{why}")
    };
    if busy {
        return refuse(AckRefusal::Busy, "Formiga Farm is already open".into());
    }
    let snapshot: FarmSnapshot = match decode(&bytes) {
        Ok(snapshot) => snapshot,
        Err(FarmError::UnsupportedVersion { needs, reads }) => {
            return refuse(
                AckRefusal::UnsupportedVersion {
                    reads: FARM_FORMAT_VERSION,
                },
                format!("Desktop needs Farm version {needs}; this Farm reads up to {reads}"),
            );
        }
        Err(error) => return refuse(AckRefusal::Invalid, format!("the snapshot: {error}")),
    };
    if snapshot.session_id != session {
        return refuse(
            AckRefusal::Invalid,
            "the snapshot is for another session".into(),
        );
    }
    if let Err(error) = snapshot.base_genome() {
        return refuse(AckRefusal::Invalid, format!("the snapshot: {error}"));
    }
    write_document(&dir.join(ACK_FILE), &FarmAck::accepted(&seal, FARM_VERSION))?;
    // Carry on from any proposal an earlier window of this session already made.
    let serial = read_document::<FarmProposal>(&dir.join(PROPOSAL_FILE))
        .map(|(proposal, _)| proposal.serial)
        .unwrap_or(0);
    Ok((
        Visit {
            dir: dir.to_owned(),
            seal,
            serial,
        },
        snapshot,
    ))
}

impl Host {
    /// Offer `design` to Desktop. Returns the serial it was written as.
    pub fn propose(
        &mut self,
        snapshot: &FarmSnapshot,
        kind: ProposalKind,
        design: &Design,
        lineage: Lineage,
    ) -> Result<u32> {
        match self {
            Self::Desktop(visit) => {
                visit.serial += 1;
                let proposal = FarmProposal::new(
                    &visit.seal,
                    visit.serial,
                    now(),
                    FARM_VERSION,
                    kind,
                    design.clone(),
                    lineage,
                );
                write_document(&visit.dir.join(PROPOSAL_FILE), &proposal)
                    .context("could not hand the design to Desktop")?;
                Ok(visit.serial)
            }
            Self::Rehearsal(rehearsal) => rehearsal.propose(snapshot, kind, design, lineage),
        }
    }

    /// Look once for anything Desktop has said about proposal `serial`, or about the session.
    pub fn listen(&mut self, serial: Option<u32>) -> Heard {
        match self {
            Self::Desktop(visit) => {
                if visit.dir.join(RECALL_FILE).exists() || !visit.dir.join(SNAPSHOT_FILE).exists() {
                    if let Ok((recall, _)) =
                        read_document::<FarmRecall>(&visit.dir.join(RECALL_FILE))
                        && recall.session_id != visit.seal.session_id
                    {
                        return Heard::Nothing;
                    }
                    return Heard::Recalled;
                }
                let Some(serial) = serial else {
                    return Heard::Nothing;
                };
                match read_document::<FarmVerdict>(&visit.dir.join(VERDICT_FILE)) {
                    Ok((verdict, _)) if verdict.answers(&visit.seal, serial) => {
                        Heard::Verdict(verdict.verdict)
                    }
                    _ => Heard::Nothing,
                }
            }
            Self::Rehearsal(rehearsal) => match serial {
                Some(serial) => rehearsal
                    .verdicts
                    .iter()
                    .find(|(s, _)| *s == serial)
                    .map_or(Heard::Nothing, |(_, verdict)| {
                        Heard::Verdict(verdict.clone())
                    }),
                None => Heard::Nothing,
            },
        }
    }

    pub fn is_rehearsal(&self) -> bool {
        matches!(self, Self::Rehearsal(_))
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Desktop(_) => None,
            Self::Rehearsal(rehearsal) => Some(&rehearsal.label),
        }
    }
}

/// The time now, to the whole second: what Farm stamps its documents, drafts and presets with.
pub fn now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(0).unwrap_or(now)
}

/// Desktop, played by Farm itself: a colony read once and never written, a snapshot projected
/// from it exactly as Desktop would, and every proposal decided by the contract's own
/// [`accept_proposal`] and kept with its own [`apply_design`], so a rehearsal proves the same
/// loop a real session does. What it keeps lasts only as long as the window.
pub struct Rehearsal {
    pub label: String,
    colony: SaveFile,
    creature: Option<CreatureId>,
    sculpt: Option<Sculpt>,
    seal: SessionSeal,
    serial: u32,
    verdicts: Vec<(u32, Verdict)>,
    /// How many new companions the rehearsal has welcomed.
    pub welcomed: usize,
}

impl Rehearsal {
    /// A rehearsal of reshaping creature `index` of `colony`, or of drawing a new one.
    pub fn open(
        colony: SaveFile,
        index: Option<usize>,
        label: String,
    ) -> Result<(Self, FarmSnapshot)> {
        let renderer = formiga_farm_contract::Renderer::this_build();
        let capabilities = formiga_farm_contract::sample::capabilities();
        let session = SessionId::parse(match index {
            Some(_) => formiga_farm_contract::sample::EDIT_SESSION,
            None => formiga_farm_contract::sample::CREATE_SESSION,
        })
        .expect("a session id");
        let (snapshot, creature) = match index {
            Some(index) => {
                let creature = colony
                    .creatures
                    .get(index)
                    .with_context(|| format!("the colony has no creature {index}"))?
                    .id;
                let snapshot = formiga_farm_contract::project_edit(
                    &colony,
                    creature,
                    None,
                    true,
                    session,
                    now(),
                    "rehearsal",
                    capabilities,
                    renderer,
                )?;
                (snapshot, Some(creature))
            }
            None => {
                let mut stand_in = formiga_farm_contract::sample::stand_in();
                if let Some(first) = colony.creatures.first() {
                    stand_in.face = first.appearance.face;
                }
                let snapshot = formiga_farm_contract::project_create(
                    &colony,
                    &stand_in,
                    session,
                    now(),
                    "rehearsal",
                    capabilities,
                    renderer,
                )?;
                (snapshot, None)
            }
        };
        let bytes = formiga_farm_contract::encode(&snapshot)?;
        let seal = SessionSeal::of(&snapshot, &bytes);
        Ok((
            Self {
                label,
                colony,
                creature,
                sculpt: None,
                seal,
                serial: 0,
                verdicts: Vec::new(),
                welcomed: 0,
            },
            snapshot,
        ))
    }

    fn propose(
        &mut self,
        snapshot: &FarmSnapshot,
        kind: ProposalKind,
        design: &Design,
        lineage: Lineage,
    ) -> Result<u32> {
        self.serial += 1;
        let proposal = FarmProposal::new(
            &self.seal,
            self.serial,
            now(),
            FARM_VERSION,
            kind,
            design.clone(),
            lineage,
        );
        // Written and read back, as Desktop would read it.
        let proposal: FarmProposal = decode(&formiga_farm_contract::encode(&proposal)?)?;
        proposal.validate()?;
        let current = match self.creature {
            Some(id) => match self.colony.creatures.iter().find(|c| c.id == id) {
                Some(creature) => Current::Creature {
                    creature,
                    sculpt: self.sculpt.as_ref(),
                    available: true,
                },
                None => Current::Gone,
            },
            None => Current::Nobody,
        };
        let verdict = match accept_proposal(&self.seal, snapshot, &proposal, current) {
            Ok(Accepted::Edit { target, design }) => {
                let creature = self
                    .colony
                    .creatures
                    .iter_mut()
                    .find(|c| c.id == target)
                    .expect("accepted for a creature in the colony");
                self.sculpt = apply_design(creature, &design);
                Verdict::Kept {
                    revision: Design::of(&creature.appearance, self.sculpt.as_ref()).revision(),
                }
            }
            Ok(Accepted::Create { .. }) => {
                self.welcomed += 1;
                Verdict::Welcomed
            }
            Err(rejection) => rejection.verdict(),
        };
        self.verdicts.push((self.serial, verdict));
        Ok(self.serial)
    }
}

/// The colony a rehearsal opens: Desktop's sample, or a colony file read once and never written.
pub fn colony_from(save: Option<&Path>) -> Result<SaveFile> {
    match save {
        None => Ok(formiga_farm_contract::sample::colony()),
        Some(path) => Ok(formiga_core::SaveStore::read_snapshot(path)
            .with_context(|| format!("could not read the colony in {}", path.display()))?
            .into_inner()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formiga_farm_contract::{FarmMode, encode};

    fn scratch_session(snapshot: &FarmSnapshot) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("formiga-farm-session-{}", crate::store::new_id()));
        let dir = root.join(snapshot.session_id.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        write_document(&dir.join(SNAPSHOT_FILE), snapshot).unwrap();
        dir
    }

    #[test]
    fn a_session_is_answered_once_and_its_proposals_carry_on_from_the_last() {
        let snapshot = formiga_farm_contract::sample::edit_snapshot(0);
        let dir = scratch_session(&snapshot);
        let (visit, read) = arrive(&dir, false).unwrap();
        assert_eq!(read, snapshot);
        let (ack, _) = read_document::<FarmAck>(&dir.join(ACK_FILE)).unwrap();
        assert!(ack.accepted);
        let mut host = Host::Desktop(visit);
        let creature = snapshot.creature().unwrap();
        let design = creature.design().unwrap();
        let kind = ProposalKind::EditExisting {
            target: creature.id,
            expected_revision: creature.revision.clone(),
        };
        assert_eq!(
            host.propose(&snapshot, kind.clone(), &design, Lineage::default())
                .unwrap(),
            1
        );
        assert_eq!(host.listen(Some(1)), Heard::Nothing);
        // Desktop answers.
        let bytes = encode(&snapshot).unwrap();
        let seal = SessionSeal::of(&snapshot, &bytes);
        let verdict = FarmVerdict::new(&seal, 1, now(), Verdict::Unavailable);
        write_document(&dir.join(VERDICT_FILE), &verdict).unwrap();
        assert_eq!(host.listen(Some(1)), Heard::Verdict(Verdict::Unavailable));
        // A verdict for another serial is not this one's.
        assert_eq!(host.listen(Some(2)), Heard::Nothing);
        // A second window on the same session goes on from the last serial.
        let (visit, _) = arrive(&dir, false).unwrap();
        let mut again = Host::Desktop(visit);
        assert_eq!(
            again
                .propose(&snapshot, kind, &design, Lineage::default())
                .unwrap(),
            2
        );
        // A recall ends it.
        write_document(
            &dir.join(RECALL_FILE),
            &FarmRecall::new(
                snapshot.session_id.clone(),
                formiga_farm_contract::RecallReason::Closing,
            ),
        )
        .unwrap();
        assert_eq!(again.listen(None), Heard::Recalled);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_session_farm_cannot_open_is_refused_in_writing() {
        let snapshot = formiga_farm_contract::sample::edit_snapshot(0);
        let dir = scratch_session(&snapshot);
        assert!(arrive(&dir, true).is_err());
        let (ack, _) = read_document::<FarmAck>(&dir.join(ACK_FILE)).unwrap();
        assert_eq!(ack.refusal, Some(AckRefusal::Busy));
        // A snapshot from a newer Desktop is refused for its version.
        let mut value = serde_json::to_value(&snapshot).unwrap();
        value["version"] = 9.into();
        value["min_reader_version"] = 9.into();
        std::fs::write(dir.join(SNAPSHOT_FILE), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(arrive(&dir, false).is_err());
        let (ack, _) = read_document::<FarmAck>(&dir.join(ACK_FILE)).unwrap();
        assert_eq!(
            ack.refusal,
            Some(AckRefusal::UnsupportedVersion {
                reads: FARM_FORMAT_VERSION
            })
        );
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_rehearsal_keeps_a_new_look_by_the_contracts_own_rules() {
        let (rehearsal, snapshot) = Rehearsal::open(
            formiga_farm_contract::sample::colony(),
            Some(0),
            "test".into(),
        )
        .unwrap();
        let mut host = Host::Rehearsal(Box::new(rehearsal));
        let FarmMode::EditExisting { creature } = &snapshot.mode else {
            panic!()
        };
        let design = crate::presets::find("animal.orca").unwrap().design;
        let kind = ProposalKind::EditExisting {
            target: creature.id,
            expected_revision: creature.revision.clone(),
        };
        let serial = host
            .propose(&snapshot, kind.clone(), &design, Lineage::default())
            .unwrap();
        let Heard::Verdict(Verdict::Kept { revision }) = host.listen(Some(serial)) else {
            panic!("kept")
        };
        assert_eq!(revision, design.revision());
        // Proposing again from the old revision is now stale.
        let serial = host
            .propose(&snapshot, kind, &design, Lineage::default())
            .unwrap();
        assert!(matches!(
            host.listen(Some(serial)),
            Heard::Verdict(Verdict::Stale { .. })
        ));
    }
}
