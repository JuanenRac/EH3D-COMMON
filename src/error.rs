//! The errors of the project container and of its metadata.

use std::io;

/// Everything that can go wrong while reading, checking or writing a project.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file system refused a read or a write.
    #[error("input/output error: {0}")]
    Io(#[from] io::Error),
    /// `project.json` or `integrity.json` is not valid JSON of the expected shape.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The ZIP container itself is damaged or uses a feature this crate does not read.
    #[error("invalid container: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// The project file was written by a format this version cannot read.
    #[error("unsupported project format {found}: this version reads {supported}")]
    UnsupportedFormat {
        /// The version the file declares.
        found: String,
        /// What this version of the crate reads, for example `1.x`.
        supported: String,
    },
    /// An entry that must exist in the package is not there.
    #[error("the package has no entry '{0}'")]
    MissingEntry(String),
    /// An entry does not match its recorded checksum, or is not recorded at all.
    #[error("integrity check failed for '{path}': {reason}")]
    Integrity {
        /// The entry that failed.
        path: String,
        /// Why: wrong hash, wrong size, or not listed.
        reason: String,
    },
    /// An entry name could leave the package folder or uses a form that is not portable.
    #[error("unsafe entry name '{0}'")]
    UnsafePath(String),
    /// Two entries have the same name.
    #[error("duplicate entry '{0}'")]
    DuplicateEntry(String),
    /// A package is larger than the limits that protect against decompression bombs.
    #[error("limit exceeded: {0}")]
    TooLarge(String),
    /// The metadata is readable but inconsistent (an empty name, a repeated tool id, a NaN).
    #[error("invalid project metadata: {0}")]
    InvalidMetadata(String),
}

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
