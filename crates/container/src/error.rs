//! What the reader and the writer of the container report.

use std::fmt;
use std::io;

use eh3d_diagnostic::Diagnostic;

use crate::codes::PkgCode;

/// A container refused for a reason that has a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The code.
    pub code: PkgCode,
    /// What exactly was wrong, in English. May contain names taken from the package.
    pub detail: String,
    /// The entry it concerns, when there is one.
    pub entry: Option<String>,
}

impl Violation {
    /// A violation of `code` with a detail.
    pub fn new(code: PkgCode, detail: impl Into<String>) -> Violation {
        Violation {
            code,
            detail: detail.into(),
            entry: None,
        }
    }

    /// The same violation, tied to an entry.
    pub fn at(mut self, entry: impl Into<String>) -> Violation {
        self.entry = Some(entry.into());
        self
    }

    /// The diagnostic of the violation.
    pub fn diagnostic(&self) -> Diagnostic {
        self.code.diagnostic(&self.detail, self.entry.as_deref())
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.code.title())?;
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for Violation {}

/// Everything that can stop the reader or the writer.
#[derive(Debug)]
pub enum ContainerError {
    /// The container was refused for a reason with a code.
    Rejected(Violation),
    /// The file system or the stream failed for a reason that says nothing about the package.
    Io(io::Error),
}

impl ContainerError {
    /// The code, when the container was refused.
    pub fn code(&self) -> Option<PkgCode> {
        match self {
            ContainerError::Rejected(v) => Some(v.code),
            ContainerError::Io(_) => None,
        }
    }
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::Rejected(v) => v.fmt(f),
            ContainerError::Io(e) => write!(f, "input/output error: {e}"),
        }
    }
}

impl std::error::Error for ContainerError {}

impl From<Violation> for ContainerError {
    fn from(v: Violation) -> Self {
        ContainerError::Rejected(v)
    }
}

impl From<io::Error> for ContainerError {
    fn from(e: io::Error) -> Self {
        // A violation raised inside a stream travels as the payload of an io error.
        match violation_in(&e) {
            Some(v) => ContainerError::Rejected(v.clone()),
            None => ContainerError::Io(e),
        }
    }
}

/// The violation carried by an io error raised while reading a stream, if there is one.
pub fn violation_in(error: &io::Error) -> Option<&Violation> {
    error.get_ref().and_then(|e| e.downcast_ref::<Violation>())
}

pub(crate) fn stream_error(violation: Violation) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, violation)
}
