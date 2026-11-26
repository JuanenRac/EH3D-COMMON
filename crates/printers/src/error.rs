//! The typed errors, with the stable codes `EH3D-PRN-001` to `EH3D-PRN-009`. A code never changes
//! its meaning and is never reused; its title in the eight languages is [`PrnCode::title_in`].

use std::fmt;

use thiserror::Error;

/// The stable code of a kind of error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrnCode {
    /// 001: a data file is not valid JSON of the schema.
    Parse = 1,
    /// 002: a data file has a schema version this crate does not read.
    SchemaVersion = 2,
    /// 003: two records have the same identifier.
    DuplicateId = 3,
    /// 004: the identifier does not match the maker and the model, or the file.
    IdMismatch = 4,
    /// 005: a value is outside the range it can have.
    OutOfRange = 5,
    /// 006: a source is missing, is not valid, or does not cover a claim.
    Source = 6,
    /// 007: the values of a record contradict each other.
    Inconsistent = 7,
    /// 008: a text is empty, too long or has a character that is refused.
    Text = 8,
    /// 009: a data file is too large or has too many records.
    TooLarge = 9,
}

impl PrnCode {
    /// Every code, in order.
    pub const ALL: [PrnCode; 9] = [
        PrnCode::Parse,
        PrnCode::SchemaVersion,
        PrnCode::DuplicateId,
        PrnCode::IdMismatch,
        PrnCode::OutOfRange,
        PrnCode::Source,
        PrnCode::Inconsistent,
        PrnCode::Text,
        PrnCode::TooLarge,
    ];

    /// The code as text, for example `EH3D-PRN-005`.
    pub fn code(self) -> &'static str {
        match self {
            PrnCode::Parse => "EH3D-PRN-001",
            PrnCode::SchemaVersion => "EH3D-PRN-002",
            PrnCode::DuplicateId => "EH3D-PRN-003",
            PrnCode::IdMismatch => "EH3D-PRN-004",
            PrnCode::OutOfRange => "EH3D-PRN-005",
            PrnCode::Source => "EH3D-PRN-006",
            PrnCode::Inconsistent => "EH3D-PRN-007",
            PrnCode::Text => "EH3D-PRN-008",
            PrnCode::TooLarge => "EH3D-PRN-009",
        }
    }
}

impl fmt::Display for PrnCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Everything the loading and the checks can report.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{code}: {at}: {reason}")]
pub struct PrnError {
    /// The kind.
    pub code: PrnCode,
    /// The data file or the identifier of the record.
    pub at: String,
    /// What is wrong.
    pub reason: String,
}

impl PrnError {
    /// A new error.
    pub fn new(code: PrnCode, at: impl Into<String>, reason: impl Into<String>) -> PrnError {
        PrnError {
            code,
            at: at.into(),
            reason: reason.into(),
        }
    }
}
