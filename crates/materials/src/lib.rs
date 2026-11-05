//! # eh3d-materials
//!
//! The database of board materials of the suite: laminates, prepregs, flex, metal cores, PTFE and
//! ceramic filled boards, each with the values its manufacturer publishes. The design tool reads
//! the whole record (RF simulation, impedance, stack-ups, thermal work); the machine program
//! reads a small view of it ([`summary::BungardSummary`]).
//!
//! * [`model`]: the records, as written in the JSON files (a field the schema does not know is
//!   refused, a value the sheet does not publish is `null`);
//! * [`data`]: the data files, embedded in the crate, one per manufacturer;
//! * [`validate`]: the rules the data follows (unique identifiers that agree with the
//!   manufacturer and the name, believable Dk, Df, Tg and Td, a plain file name as source);
//! * [`query`]: the [`Database`], the lookups by identifier, manufacturer and kind and the search
//!   by minimum Tg and by Dk and Df;
//! * [`interp`]: Dk and Df at a frequency, interpolated between the published points and never
//!   extrapolated;
//! * [`documents`]: the catalogue of the reference documents, classified by category and by
//!   what they serve (the machine program or the design tool);
//! * [`error`]: the typed errors with the stable codes `EH3D-MAT-001` to `EH3D-MAT-009`.
//!
//! Nothing here opens a window, a file or the network: the data is part of the library.

#![forbid(unsafe_code)]

pub mod data;
pub mod documents;
pub mod error;
pub mod interp;
pub mod model;
pub mod query;
pub mod summary;
pub mod validate;

pub use documents::{Category, Consumer, DocType, Document, Relevance, Status};
pub use error::{MatCode, MatError};
pub use interp::{interpolate, published_range};
pub use model::{Kind, Material, MaterialFile, SCHEMA_VERSION};
pub use query::{parse_file, Criteria, Database};
pub use summary::BungardSummary;
