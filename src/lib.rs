//! # eh3d-common
//!
//! The data every program of the EH3D Fabrication Suite shares:
//!
//! * [`project`]: the metadata of a project and the rules of its format versions;
//! * [`container`]: the `.eh3dproj` package, a ZIP archive with a SHA-256 record of every entry;
//! * [`telemetry`]: axes, spindle, machine states and alarm codes as machines report them;
//! * [`theme`]: the colours of the suite.
//!
//! Nothing here opens a window, talks to a machine or reads the network.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod container;
pub mod error;
pub mod project;
pub mod telemetry;
pub mod theme;

pub use container::Package;
pub use error::{Error, Result};
pub use project::{ProjectMetadata, FORMAT_VERSION};
