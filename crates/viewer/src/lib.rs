//! The viewer of the attachments of a project (datasheets, manuals and other files), shared by the
//! programs of the suite. Every attachment is untrusted input.
//!
//! * [`kind`] says what a file is, from its name and its first bytes, and whether it is dangerous.
//! * [`viewer`] is the state of the panel, with no window in it: what is shown, what a person has to
//!   agree to, and what the program is asked to do. [`gui`] (feature `gui`) draws it with egui.
//! * [`pdf`] shows the pages of a PDF with PDFium, a library loaded from the folder of the program
//!   only, at a pinned version and SHA-256. Without it the viewer says so and offers to open outside.
//! * [`office`], [`markdown`], [`plain`] and [`picture`] give the previews that are drawn inside.
//! * [`outside`] makes the temporary copy and asks the system to open it. An executable or a script is
//!   never launched.
//!
//! Nothing here goes to the network, and nothing of an attachment is run.

pub mod doc;
pub mod error;
#[cfg(feature = "gui")]
pub mod gui;
pub mod kind;
pub mod limits;
pub mod markdown;
pub mod office;
pub mod outside;
pub mod pdf;
pub mod picture;
pub mod plain;
pub mod source;
pub mod texts;
pub mod viewer;
mod xml;

pub use doc::{Block, Preview};
pub use error::ViewerError;
pub use kind::{classify, inspect, Classification, Danger, Kind};
pub use limits::ViewerLimits;
pub use source::{FileSource, MemorySource, Source};
pub use viewer::{Content, Viewer, ViewerEvent};
