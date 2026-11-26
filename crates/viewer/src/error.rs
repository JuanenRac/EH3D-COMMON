//! Why something of an attachment cannot be shown or done.

use std::fmt;

/// A refusal or a failure of the viewer. The words for a person are in [`crate::texts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerError {
    /// The attachment, or a part of it, is larger than the limit given (in bytes).
    TooLarge { limit: u64 },
    /// The file is not what it says it is, or is broken.
    Damaged(String),
    /// There is no preview for this kind of file.
    Unsupported,
    /// The PDF library is not in the folder of the program.
    EngineMissing,
    /// A library of that name is there but it is not the pinned one, or it cannot be loaded.
    EngineRejected(String),
    /// The PDF asks for a password.
    PasswordProtected,
    /// A person has to agree first.
    ConsentRequired,
    /// Never done, with or without agreement (a program or a script is never launched).
    Refused,
    /// The operating system answered with an error.
    Io(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewerError::TooLarge { limit } => write!(f, "larger than the limit of {limit} bytes"),
            ViewerError::Damaged(why) => write!(f, "damaged or not the kind it says: {why}"),
            ViewerError::Unsupported => f.write_str("no preview for this kind of file"),
            ViewerError::EngineMissing => {
                f.write_str("the PDF library is not in the program folder")
            }
            ViewerError::EngineRejected(why) => write!(f, "the PDF library was not loaded: {why}"),
            ViewerError::PasswordProtected => f.write_str("the document asks for a password"),
            ViewerError::ConsentRequired => f.write_str("a person has to confirm first"),
            ViewerError::Refused => f.write_str("a program or a script is never launched"),
            ViewerError::Io(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ViewerError {}

impl From<std::io::Error> for ViewerError {
    fn from(e: std::io::Error) -> Self {
        ViewerError::Io(e.to_string())
    }
}
