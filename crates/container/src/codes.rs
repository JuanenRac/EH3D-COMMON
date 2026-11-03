//! The stable codes of the container: `EH3D-PKG-001` to `EH3D-PKG-014`.
//!
//! A code never changes its meaning and is never reused. The registry that the user reads lives
//! in the documentation repository of the suite; a test here keeps the two lists in step when the
//! documentation checkout is next to this one.

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
}

impl PkgCode {
    /// Every code, in order.
    pub const ALL: [PkgCode; 14] = [
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
    ];

    /// The number part, 1 to 14.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// The code as text, for example `EH3D-PKG-010`.
    pub fn code(self) -> &'static str {
        const CODES: [&str; 14] = [
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
        ];
        CODES[self as usize - 1]
    }

    /// The code that has this text, if any.
    pub fn from_code(text: &str) -> Option<PkgCode> {
        PkgCode::ALL.into_iter().find(|c| c.code() == text)
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
        }
    }

    /// A diagnostic for this code with a detail, and the entry it concerns when there is one.
    /// The detail may contain names taken from the package; the redacted form of the diagnostic
    /// drops it.
    pub fn diagnostic(self, detail: &str, entry: Option<&str>) -> Diagnostic {
        let message = if detail.is_empty() {
            self.title().to_owned()
        } else {
            format!("{}: {detail}", self.title())
        };
        let mut diagnostic = Diagnostic::new(0, Severity::Error, message)
            .with_code(self.code())
            .with_category(Category::Format)
            .with_recoverability(Recoverability::Unrecoverable);
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
    fn the_fourteen_codes_are_numbered_without_gaps() {
        for (index, code) in PkgCode::ALL.iter().enumerate() {
            assert_eq!(code.number() as usize, index + 1);
            assert_eq!(code.code(), format!("EH3D-PKG-{:03}", index + 1));
            assert_eq!(PkgCode::from_code(code.code()), Some(*code));
            assert!(!code.title().is_empty());
        }
        assert_eq!(PkgCode::from_code("EH3D-PKG-015"), None);
        assert_eq!(PkgCode::from_code("EH3D-PKG-000"), None);
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
