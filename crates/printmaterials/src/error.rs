//! What the database reports: typed errors with the stable codes `EH3D-PMAT-001` to `EH3D-PMAT-009`.
//!
//! A code never changes its meaning and is never reused. The English title is here; the titles in
//! the eight languages of the suite are in [`crate::i18n`] (the user interface asks for them by
//! the key `diag.<code>`, as for the codes of the container).

use std::fmt;

use thiserror::Error;

/// The stable code of a kind of error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PmCode {
    /// 001: a data file is not valid JSON of the schema.
    Parse = 1,
    /// 002: a data file has a schema version this crate does not read.
    SchemaVersion = 2,
    /// 003: two records have the same identifier.
    DuplicateId = 3,
    /// 004: the identifier does not match the manufacturer and the name.
    IdMismatch = 4,
    /// 005: a value is outside the range it can have.
    OutOfRange = 5,
    /// 006: the source of a record is missing or is not valid.
    Source = 6,
    /// 007: a data file of the folder is not in the list of embedded files.
    FileList = 7,
    /// 008: the exposure of a resin cannot be given for a printer and a layer.
    Exposure = 8,
    /// 009: the values of a record contradict each other (a lowest above a highest...).
    Inconsistent = 9,
}

impl PmCode {
    /// Every code, in order.
    pub const ALL: [PmCode; 9] = [
        PmCode::Parse,
        PmCode::SchemaVersion,
        PmCode::DuplicateId,
        PmCode::IdMismatch,
        PmCode::OutOfRange,
        PmCode::Source,
        PmCode::FileList,
        PmCode::Exposure,
        PmCode::Inconsistent,
    ];

    /// The number part, 1 to 9.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// The code as text, for example `EH3D-PMAT-005`.
    pub fn code(self) -> &'static str {
        match self {
            PmCode::Parse => "EH3D-PMAT-001",
            PmCode::SchemaVersion => "EH3D-PMAT-002",
            PmCode::DuplicateId => "EH3D-PMAT-003",
            PmCode::IdMismatch => "EH3D-PMAT-004",
            PmCode::OutOfRange => "EH3D-PMAT-005",
            PmCode::Source => "EH3D-PMAT-006",
            PmCode::FileList => "EH3D-PMAT-007",
            PmCode::Exposure => "EH3D-PMAT-008",
            PmCode::Inconsistent => "EH3D-PMAT-009",
        }
    }

    /// The code that has this text, if any.
    pub fn from_code(text: &str) -> Option<PmCode> {
        PmCode::ALL.into_iter().find(|c| c.code() == text)
    }

    /// The English title of the code.
    pub fn title(self) -> &'static str {
        match self {
            PmCode::Parse => "Data file not valid",
            PmCode::SchemaVersion => "Schema version not supported",
            PmCode::DuplicateId => "Duplicate identifier",
            PmCode::IdMismatch => "Identifier does not match the record",
            PmCode::OutOfRange => "Value out of range",
            PmCode::Source => "Source missing or not valid",
            PmCode::FileList => "Data file not in the embedded list",
            PmCode::Exposure => "Exposure cannot be given",
            PmCode::Inconsistent => "Values contradict each other",
        }
    }
}

impl fmt::Display for PmCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Everything the loading and the checks can report.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PmError {
    /// A data file is not valid JSON of the schema.
    #[error("EH3D-PMAT-001: data file {file} is not valid: {message}")]
    Parse {
        /// The data file.
        file: String,
        /// What the parser said.
        message: String,
    },
    /// A data file has a schema version this crate does not read.
    #[error(
        "EH3D-PMAT-002: data file {file} has schema version {found}, this crate reads {supported}"
    )]
    SchemaVersion {
        /// The data file.
        file: String,
        /// The version found.
        found: u32,
        /// The version supported.
        supported: u32,
    },
    /// Two records have the same identifier.
    #[error("EH3D-PMAT-003: the identifier {id} appears more than once")]
    DuplicateId {
        /// The identifier.
        id: String,
    },
    /// The identifier does not match the manufacturer and the name.
    #[error("EH3D-PMAT-004: the identifier {id} does not match: {reason}")]
    IdMismatch {
        /// The identifier.
        id: String,
        /// What does not match.
        reason: String,
    },
    /// A value is outside the range it can have.
    #[error("EH3D-PMAT-005: {id}: {field} = {value} is outside {min} to {max}")]
    OutOfRange {
        /// The record.
        id: String,
        /// The field.
        field: String,
        /// The value.
        value: f64,
        /// The lowest value allowed.
        min: f64,
        /// The highest value allowed.
        max: f64,
    },
    /// The source of a record is missing or is not valid.
    #[error("EH3D-PMAT-006: {id}: {reason}")]
    Source {
        /// The record.
        id: String,
        /// What is wrong.
        reason: String,
    },
    /// A data file of the folder is not in the list of embedded files.
    #[error("EH3D-PMAT-007: the data file {file} is not in the embedded list")]
    FileList {
        /// The file.
        file: String,
    },
    /// The exposure of a resin cannot be given for a printer and a layer.
    #[error("EH3D-PMAT-008: {id}: {reason}")]
    Exposure {
        /// The resin.
        id: String,
        /// Why not.
        reason: String,
    },
    /// The values of a record contradict each other.
    #[error("EH3D-PMAT-009: {id}: {reason}")]
    Inconsistent {
        /// The record.
        id: String,
        /// What contradicts what.
        reason: String,
    },
}

impl PmError {
    /// The stable code of the error.
    pub fn code(&self) -> PmCode {
        match self {
            PmError::Parse { .. } => PmCode::Parse,
            PmError::SchemaVersion { .. } => PmCode::SchemaVersion,
            PmError::DuplicateId { .. } => PmCode::DuplicateId,
            PmError::IdMismatch { .. } => PmCode::IdMismatch,
            PmError::OutOfRange { .. } => PmCode::OutOfRange,
            PmError::Source { .. } => PmCode::Source,
            PmError::FileList { .. } => PmCode::FileList,
            PmError::Exposure { .. } => PmCode::Exposure,
            PmError::Inconsistent { .. } => PmCode::Inconsistent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_codes_are_numbered_without_gaps() {
        for (index, code) in PmCode::ALL.iter().enumerate() {
            assert_eq!(code.number() as usize, index + 1);
            assert_eq!(code.code(), format!("EH3D-PMAT-{:03}", index + 1));
            assert_eq!(PmCode::from_code(code.code()), Some(*code));
            assert!(!code.title().is_empty());
        }
        assert_eq!(PmCode::from_code("EH3D-PMAT-010"), None);
        assert_eq!(PmCode::from_code("EH3D-MAT-001"), None);
    }

    #[test]
    fn an_error_reads_with_its_code() {
        let e = PmError::DuplicateId { id: "a/b".into() };
        assert_eq!(e.code(), PmCode::DuplicateId);
        assert!(e.to_string().starts_with("EH3D-PMAT-003"));
        let e = PmError::Inconsistent {
            id: "a/b".into(),
            reason: "x".into(),
        };
        assert!(e.to_string().starts_with(e.code().code()));
    }
}
