//! What the database reports: typed errors with the stable codes `EH3D-MAT-001` to `EH3D-MAT-008`.
//!
//! A code never changes its meaning and is never reused. The titles are in English; the user
//! interface translates them by the key `diag.<code>`, as for the codes of the container.

use std::fmt;

use thiserror::Error;

/// The stable code of a kind of error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatCode {
    /// 001: a data file is not valid JSON of the schema.
    Parse = 1,
    /// 002: a data file has a schema version this crate does not read.
    SchemaVersion = 2,
    /// 003: two materials have the same identifier.
    DuplicateId = 3,
    /// 004: the identifier does not match the manufacturer and the name, or the file.
    IdMismatch = 4,
    /// 005: a value is outside the range it can have.
    OutOfRange = 5,
    /// 006: the source of a material is missing or is not a plain file name.
    Source = 6,
    /// 007: a data file of the folder is not in the list of embedded files.
    FileList = 7,
    /// 008: a value cannot be interpolated.
    Interpolation = 8,
}

impl MatCode {
    /// Every code, in order.
    pub const ALL: [MatCode; 8] = [
        MatCode::Parse,
        MatCode::SchemaVersion,
        MatCode::DuplicateId,
        MatCode::IdMismatch,
        MatCode::OutOfRange,
        MatCode::Source,
        MatCode::FileList,
        MatCode::Interpolation,
    ];

    /// The number part, 1 to 8.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// The code as text, for example `EH3D-MAT-005`.
    pub fn code(self) -> &'static str {
        match self {
            MatCode::Parse => "EH3D-MAT-001",
            MatCode::SchemaVersion => "EH3D-MAT-002",
            MatCode::DuplicateId => "EH3D-MAT-003",
            MatCode::IdMismatch => "EH3D-MAT-004",
            MatCode::OutOfRange => "EH3D-MAT-005",
            MatCode::Source => "EH3D-MAT-006",
            MatCode::FileList => "EH3D-MAT-007",
            MatCode::Interpolation => "EH3D-MAT-008",
        }
    }

    /// The code that has this text, if any.
    pub fn from_code(text: &str) -> Option<MatCode> {
        MatCode::ALL.into_iter().find(|c| c.code() == text)
    }

    /// The English title of the code (the user interface translates by the key `diag.<code>`).
    pub fn title(self) -> &'static str {
        match self {
            MatCode::Parse => "Data file not valid",
            MatCode::SchemaVersion => "Schema version not supported",
            MatCode::DuplicateId => "Duplicate material identifier",
            MatCode::IdMismatch => "Identifier does not match the material",
            MatCode::OutOfRange => "Value out of range",
            MatCode::Source => "Source missing or not a file name",
            MatCode::FileList => "Data file not in the embedded list",
            MatCode::Interpolation => "Value cannot be interpolated",
        }
    }
}

impl fmt::Display for MatCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Everything the loading, the checks and the interpolation can report.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum MatError {
    /// A data file is not valid JSON of the schema.
    #[error("EH3D-MAT-001: data file {file} is not valid: {message}")]
    Parse {
        /// The data file.
        file: String,
        /// What the parser said.
        message: String,
    },
    /// A data file has a schema version this crate does not read.
    #[error(
        "EH3D-MAT-002: data file {file} has schema version {found}, this crate reads {supported}"
    )]
    SchemaVersion {
        /// The data file.
        file: String,
        /// The version found.
        found: u32,
        /// The version supported.
        supported: u32,
    },
    /// Two materials have the same identifier.
    #[error("EH3D-MAT-003: the identifier {id} appears more than once")]
    DuplicateId {
        /// The identifier.
        id: String,
    },
    /// The identifier does not match the manufacturer and the name, or the file.
    #[error("EH3D-MAT-004: the identifier {id} does not match: {reason}")]
    IdMismatch {
        /// The identifier.
        id: String,
        /// What does not match.
        reason: String,
    },
    /// A value is outside the range it can have.
    #[error("EH3D-MAT-005: {id}: {field} = {value} is outside {min} to {max}")]
    OutOfRange {
        /// The material.
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
    /// The source of a material is missing or is not a plain file name.
    #[error("EH3D-MAT-006: {id}: {reason}")]
    Source {
        /// The material.
        id: String,
        /// What is wrong.
        reason: String,
    },
    /// A data file of the folder is not in the list of embedded files.
    #[error("EH3D-MAT-007: the data file {file} is not in the embedded list")]
    FileList {
        /// The file.
        file: String,
    },
    /// A value cannot be interpolated.
    #[error("EH3D-MAT-008: {id}: {reason}")]
    Interpolation {
        /// The material.
        id: String,
        /// Why not.
        reason: String,
    },
}

impl MatError {
    /// The stable code of the error.
    pub fn code(&self) -> MatCode {
        match self {
            MatError::Parse { .. } => MatCode::Parse,
            MatError::SchemaVersion { .. } => MatCode::SchemaVersion,
            MatError::DuplicateId { .. } => MatCode::DuplicateId,
            MatError::IdMismatch { .. } => MatCode::IdMismatch,
            MatError::OutOfRange { .. } => MatCode::OutOfRange,
            MatError::Source { .. } => MatCode::Source,
            MatError::FileList { .. } => MatCode::FileList,
            MatError::Interpolation { .. } => MatCode::Interpolation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eight_codes_are_numbered_without_gaps() {
        for (index, code) in MatCode::ALL.iter().enumerate() {
            assert_eq!(code.number() as usize, index + 1);
            assert_eq!(code.code(), format!("EH3D-MAT-{:03}", index + 1));
            assert_eq!(MatCode::from_code(code.code()), Some(*code));
            assert!(!code.title().is_empty());
        }
        assert_eq!(MatCode::from_code("EH3D-MAT-009"), None);
    }

    #[test]
    fn an_error_reads_with_its_code() {
        let e = MatError::DuplicateId { id: "a/b".into() };
        assert_eq!(e.code(), MatCode::DuplicateId);
        assert!(e.to_string().starts_with("EH3D-MAT-003"));
    }
}
