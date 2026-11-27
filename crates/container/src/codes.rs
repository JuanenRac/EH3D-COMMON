//! The stable codes of the container: `EH3D-PKG-001` to `EH3D-PKG-026`.
//!
//! A code never changes its meaning and is never reused. The registry that the user reads lives
//! in the documentation repository of the suite; a test here keeps the two lists in step when the
//! documentation checkout is next to this one.
//!
//! Codes 001 to 014 belong to the container itself (names, ZIP structure, integrity, limits,
//! version, atomic replacement). Codes 015 to 024 belong to the project container with branches
//! (`.eh3d`, crate `eh3d-project`): branch records, branch hashes, ownership, references between
//! branches, seals, signatures and encryption. Codes 025 and 026 are refusals of a write that would
//! lose something nobody agreed to lose: the work another program saved, or a signature.

use std::fmt;

use eh3d_diagnostic::{Category, Diagnostic, Location, Recoverability, Severity};

/// One reason a container is refused. The number is the last part of the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PkgCode {
    /// 001: an entry name is not valid.
    InvalidName = 1,
    /// 002: two entries have the same name once case and Unicode form are ignored.
    DuplicateName = 2,
    /// 003: a symbolic link, device or special entry.
    SpecialEntry = 3,
    /// 004: an explicit directory entry.
    DirectoryEntry = 4,
    /// 005: ZIP64 records where the format does not allow them.
    Zip64 = 5,
    /// 006: an encrypted entry or a compression method the format does not allow.
    Method = 6,
    /// 007: a wrong checksum, a truncated archive or data after its end.
    DamagedArchive = 7,
    /// 008: general-purpose flags the format does not allow.
    Flags = 8,
    /// 009: an entry outside the roots the format allows, or a required entry that is missing.
    OutsideRoots = 9,
    /// 010: the integrity record is missing, incomplete or does not match.
    Integrity = 10,
    /// 011: a size or count limit is exceeded.
    Limit = 11,
    /// 012: the time or memory budget is exceeded.
    Budget = 12,
    /// 013: the format version is newer than this reader supports.
    NewerVersion = 13,
    /// 014: the atomic replacement of the target failed; the target is untouched.
    AtomicReplace = 14,
    /// 015: a branch record (`branch.json`), the project record or an optional record
    /// (metadata, document index, conversion report) is missing, malformed or inconsistent.
    BranchRecord = 15,
    /// 016: the content hash of a branch differs from the one its record states.
    BranchHash = 16,
    /// 017: a write to a branch that the writing application does not own.
    BranchOwner = 17,
    /// 018: a reference to another branch is malformed, points outside the file or to a branch
    /// that is not there.
    BranchRef = 18,
    /// 019: a branch was derived from a state of another branch that has changed since (a
    /// warning: the file is valid and the derived data can be updated).
    StaleDerived = 19,
    /// 020: the branch or the project is sealed read-only.
    Sealed = 20,
    /// 021: a signature does not verify.
    SignatureInvalid = 21,
    /// 022: a signature is required and missing, or the key that made it is not known.
    SignatureMissing = 22,
    /// 023: a branch is encrypted and cannot be read without its key.
    EncryptedBranch = 23,
    /// 024: the format version of a branch is newer than the application supports.
    BranchVersion = 24,
    /// 025: the file changed on disk since it was read, so nothing was written over it.
    ChangedOnDisk = 25,
    /// 026: there is a signature already and replacing it was not agreed to.
    SignatureExists = 26,
}

impl PkgCode {
    /// Every code, in order.
    pub const ALL: [PkgCode; 26] = [
        PkgCode::InvalidName,
        PkgCode::DuplicateName,
        PkgCode::SpecialEntry,
        PkgCode::DirectoryEntry,
        PkgCode::Zip64,
        PkgCode::Method,
        PkgCode::DamagedArchive,
        PkgCode::Flags,
        PkgCode::OutsideRoots,
        PkgCode::Integrity,
        PkgCode::Limit,
        PkgCode::Budget,
        PkgCode::NewerVersion,
        PkgCode::AtomicReplace,
        PkgCode::BranchRecord,
        PkgCode::BranchHash,
        PkgCode::BranchOwner,
        PkgCode::BranchRef,
        PkgCode::StaleDerived,
        PkgCode::Sealed,
        PkgCode::SignatureInvalid,
        PkgCode::SignatureMissing,
        PkgCode::EncryptedBranch,
        PkgCode::BranchVersion,
        PkgCode::ChangedOnDisk,
        PkgCode::SignatureExists,
    ];

    /// The number part, 1 to 26.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// The code as text, for example `EH3D-PKG-010`.
    pub fn code(self) -> &'static str {
        const CODES: [&str; 26] = [
            "EH3D-PKG-001",
            "EH3D-PKG-002",
            "EH3D-PKG-003",
            "EH3D-PKG-004",
            "EH3D-PKG-005",
            "EH3D-PKG-006",
            "EH3D-PKG-007",
            "EH3D-PKG-008",
            "EH3D-PKG-009",
            "EH3D-PKG-010",
            "EH3D-PKG-011",
            "EH3D-PKG-012",
            "EH3D-PKG-013",
            "EH3D-PKG-014",
            "EH3D-PKG-015",
            "EH3D-PKG-016",
            "EH3D-PKG-017",
            "EH3D-PKG-018",
            "EH3D-PKG-019",
            "EH3D-PKG-020",
            "EH3D-PKG-021",
            "EH3D-PKG-022",
            "EH3D-PKG-023",
            "EH3D-PKG-024",
            "EH3D-PKG-025",
            "EH3D-PKG-026",
        ];
        CODES[self as usize - 1]
    }

    /// The code that has this text, if any.
    pub fn from_code(text: &str) -> Option<PkgCode> {
        PkgCode::ALL.into_iter().find(|c| c.code() == text)
    }

    /// True for a code that reports something worth knowing in a file that is valid (today only
    /// 019, derived data out of date); every other code means the operation was refused.
    pub fn is_warning(self) -> bool {
        self == PkgCode::StaleDerived
    }

    /// The English title of the code (the user interface translates by the key `diag.<code>`).
    pub fn title(self) -> &'static str {
        match self {
            PkgCode::InvalidName => "Invalid entry name",
            PkgCode::DuplicateName => "Duplicate entry name",
            PkgCode::SpecialEntry => "Link, device or special entry",
            PkgCode::DirectoryEntry => "Explicit directory entry",
            PkgCode::Zip64 => "ZIP64 records not allowed",
            PkgCode::Method => "Encrypted entry or compression method not allowed",
            PkgCode::DamagedArchive => "Damaged archive",
            PkgCode::Flags => "General-purpose flags not allowed",
            PkgCode::OutsideRoots => "Entry outside the allowed roots or required entry missing",
            PkgCode::Integrity => "Integrity record missing, incomplete or different",
            PkgCode::Limit => "Limit exceeded",
            PkgCode::Budget => "Time or memory budget exceeded",
            PkgCode::NewerVersion => "Format version newer than supported",
            PkgCode::AtomicReplace => "Atomic replacement failed",
            PkgCode::BranchRecord => "Branch or project record missing or invalid",
            PkgCode::BranchHash => "Branch content hash differs from its record",
            PkgCode::BranchOwner => "Write to a branch of another application refused",
            PkgCode::BranchRef => "Reference between branches invalid or dangling",
            PkgCode::StaleDerived => "Derived data out of date",
            PkgCode::Sealed => "Branch or project sealed read-only",
            PkgCode::SignatureInvalid => "Signature does not verify",
            PkgCode::SignatureMissing => "Signature required but missing or its key unknown",
            PkgCode::EncryptedBranch => "Branch encrypted and its key not available",
            PkgCode::BranchVersion => "Branch format version newer than supported",
            PkgCode::ChangedOnDisk => "File changed on disk since it was read",
            PkgCode::SignatureExists => "Signature there already and its replacement not agreed",
        }
    }

    /// A diagnostic for this code with a detail, and the entry it concerns when there is one.
    /// The detail may contain names taken from the package; the redacted form of the diagnostic
    /// drops it. A warning code (see [`PkgCode::is_warning`]) gives a warning that the reader
    /// went past; every other code gives an error that stopped it.
    pub fn diagnostic(self, detail: &str, entry: Option<&str>) -> Diagnostic {
        let message = if detail.is_empty() {
            self.title().to_owned()
        } else {
            format!("{}: {detail}", self.title())
        };
        let (severity, recoverability) = if self.is_warning() {
            (Severity::Warning, Recoverability::Recoverable)
        } else if matches!(self, PkgCode::ChangedOnDisk | PkgCode::SignatureExists) {
            // a refusal that reading again, another name or an agreement gets past
            (Severity::Error, Recoverability::Recoverable)
        } else {
            (Severity::Error, Recoverability::Unrecoverable)
        };
        let mut diagnostic = Diagnostic::new(0, severity, message)
            .with_code(self.code())
            .with_category(Category::Format)
            .with_recoverability(recoverability);
        if let Some(entry) = entry {
            diagnostic = diagnostic
                .with_location(Location::Entry(entry.to_owned()))
                .with_arg("entry", entry);
        }
        diagnostic
    }
}

impl fmt::Display for PkgCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_twenty_six_codes_are_numbered_without_gaps() {
        for (index, code) in PkgCode::ALL.iter().enumerate() {
            assert_eq!(code.number() as usize, index + 1);
            assert_eq!(code.code(), format!("EH3D-PKG-{:03}", index + 1));
            assert_eq!(PkgCode::from_code(code.code()), Some(*code));
            assert!(!code.title().is_empty());
        }
        assert_eq!(PkgCode::from_code("EH3D-PKG-027"), None);
        assert_eq!(PkgCode::from_code("EH3D-PKG-000"), None);
    }

    #[test]
    fn the_first_fourteen_codes_keep_their_numbers_and_titles() {
        // 001 to 014 are in use by the readers of `.eh3dproj`: their meaning never changes.
        assert_eq!(PkgCode::InvalidName.number(), 1);
        assert_eq!(PkgCode::AtomicReplace.number(), 14);
        assert_eq!(
            PkgCode::Integrity.title(),
            "Integrity record missing, incomplete or different"
        );
        assert_eq!(PkgCode::BranchRecord.number(), 15);
    }

    #[test]
    fn only_the_stale_code_is_a_warning() {
        for code in PkgCode::ALL {
            let d = code.diagnostic("x", None);
            if code == PkgCode::StaleDerived {
                assert!(code.is_warning());
                assert_eq!(d.severity, Severity::Warning);
                assert_eq!(d.recoverability, Some(Recoverability::Recoverable));
            } else {
                assert!(!code.is_warning());
                assert_eq!(d.severity, Severity::Error, "{code}");
            }
        }
    }

    #[test]
    fn the_diagnostic_carries_the_code_the_entry_and_a_redacted_form() {
        let d = PkgCode::Integrity.diagnostic("size differs", Some("layers/a.gbr"));
        assert_eq!(d.code.as_deref(), Some("EH3D-PKG-010"));
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.location, Some(Location::Entry("layers/a.gbr".into())));
        assert!(d.message.starts_with("Integrity record missing"));
        let redacted = d.redacted();
        assert_eq!(redacted.code.as_deref(), Some("EH3D-PKG-010"));
        assert!(redacted.message.is_empty() && redacted.location.is_none());
        assert!(PkgCode::Limit.diagnostic("", None).message == "Limit exceeded");
    }
}
