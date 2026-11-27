//! What a project file held when it was looked at: a mark of its **content**, never of its date.
//!
//! A program keeps the [`Revision`] of the project it opened and does not write over a file that
//! no longer holds that content, because another program saved meanwhile (`EH3D-PKG-025`). The
//! mark is made from the integrity record (the name, size and SHA-256 of every entry), so a file
//! that was only touched, or packed again with the same content, is the same.
//!
//! Writes of **this** program do not count as another one: each write made through this crate is
//! remembered while the program runs (from which content to which), so the panel of attachments
//! and the window of the program may write one after the other.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use eh3d_container::{Container, ContainerError, ContainerSpec, Integrity, PkgCode, Violation};
use sha2::{Digest, Sha256};

use crate::records::to_hex;
use crate::spec::eh3d;

/// A record larger than this is not read.
const MAX_RECORD: u64 = 16 * 1024 * 1024;
/// The entry of the integrity record, which does not list itself.
const RECORD: &str = "integrity.json";
/// How many writes of this program are remembered.
const MAX_REMEMBERED: usize = 4096;

/// A write this program made: the file, what it held before and what it holds after.
struct Own {
    path: PathBuf,
    from: Revision,
    to: Revision,
}

static OWN: Mutex<Vec<Own>> = Mutex::new(Vec::new());

/// The mark of the content of a project file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Revision(String);

impl Revision {
    /// The mark of the entries an integrity record lists.
    pub fn from_integrity(integrity: &Integrity) -> Revision {
        let mut hash = Sha256::new();
        for (name, entry) in &integrity.files {
            // no name of an entry holds a zero byte, and the other two have a fixed form
            hash.update(name.as_bytes());
            hash.update([0]);
            hash.update(entry.size.to_le_bytes());
            hash.update(entry.sha256.as_bytes());
            hash.update([0]);
        }
        Revision(to_hex(&hash.finalize()))
    }

    /// The mark of the project file at `path` as it is now. Only its list of entries and its
    /// integrity record are read; a list that does not agree with the record is an error.
    pub fn of(path: &Path) -> Result<Revision, ContainerError> {
        Revision::with_spec(path, &eh3d())
    }

    pub(crate) fn with_spec(path: &Path, spec: &ContainerSpec) -> Result<Revision, ContainerError> {
        let mut container = Container::open_path(path, spec)?;
        let bytes = container.read_limited(RECORD, MAX_RECORD)?;
        let differs = || {
            ContainerError::Rejected(Violation::new(
                PkgCode::Integrity,
                "the entries are not the ones the integrity record lists",
            ))
        };
        let integrity: Integrity = serde_json::from_slice(&bytes).map_err(|_| differs())?;
        let listed =
            container
                .entries()
                .filter(|e| e.name != RECORD)
                .try_fold(0usize, |count, e| {
                    integrity
                        .files
                        .get(&e.name)
                        .filter(|r| r.size == e.size)
                        .map(|_| count + 1)
                });
        if listed != Some(integrity.files.len()) {
            return Err(differs());
        }
        Ok(Revision::from_integrity(&integrity))
    }

    /// The mark as 64 hexadecimal digits, to be kept by a program.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The mark of a text made by [`Revision::as_str`].
    pub fn parse(text: &str) -> Option<Revision> {
        let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
        (text.len() == 64 && text.bytes().all(hex)).then(|| Revision(text.to_owned()))
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Remembers a write of this program.
pub(crate) fn note_own_write(path: &Path, from: Revision, to: Revision) {
    if from == to {
        return;
    }
    let path = key(path);
    let mut own = OWN.lock().unwrap_or_else(PoisonError::into_inner);
    own.retain(|w| !(w.path == path && w.from == from));
    if own.len() >= MAX_REMEMBERED {
        own.remove(0);
    }
    own.push(Own { path, from, to });
}

/// Whether the content `now` of the file at `path` is the content `seen`, or comes from it by
/// writes of this program only.
pub fn follows(path: &Path, seen: &Revision, now: &Revision) -> bool {
    if seen == now {
        return true;
    }
    let path = key(path);
    let own = OWN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut at = seen;
    for _ in 0..=own.len() {
        match own.iter().find(|w| w.path == path && &w.from == at) {
            Some(write) if &write.to == now => return true,
            Some(write) => at = &write.to,
            None => return false,
        }
    }
    false
}

/// The refusal of a write over a file that changed.
pub(crate) fn changed() -> ContainerError {
    ContainerError::Rejected(Violation::new(
        PkgCode::ChangedOnDisk,
        "another program saved the file since it was read; nothing was written: open it again, or save under another name",
    ))
}

/// Refuses (`EH3D-PKG-025`) when the file at `path` no longer holds what was read as `seen`:
/// another program saved it, or it is gone or cannot be read.
pub fn check_unchanged(path: &Path, seen: &Revision) -> Result<(), ContainerError> {
    match Revision::of(path) {
        Ok(now) if follows(path, seen, &now) => Ok(()),
        _ => Err(changed()),
    }
}
