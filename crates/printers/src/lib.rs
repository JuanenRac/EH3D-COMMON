//! # eh3d-printers
//!
//! The catalog of 3D printers of the suite, shared by the slicer, the server and the clients.
//!
//! * [`model`]: the record of a printer, as written in the JSON files (a field the schema does
//!   not know is refused, a figure the maker does not publish is absent or `unknown`, nothing is
//!   marked verified);
//! * [`validate`]: the rules the data follows (identifiers that agree with the maker and the
//!   model, believable figures, and for every claim a public page that says it);
//! * [`catalog`]: the [`Catalog`] of the data files embedded in the crate, one per maker;
//! * [`start`]: what to start from on a printer: the generic filaments of
//!   `eh3d-printmaterials` it can take, with the reasons against the others, and the processes
//!   computed from the nozzle;
//! * [`i18n`]: every word a program shows, in the eight languages of the suite;
//! * [`error`]: the typed errors with the stable codes `EH3D-PRN-001` to `EH3D-PRN-009`.
//!
//! The figures come from the public pages of the makers and none has been tried on a machine.
//! Nothing here opens a window, a file or the network, and nothing talks to a printer.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod error;
pub mod i18n;
pub mod model;
pub mod start;
pub mod validate;

pub use catalog::Catalog;
pub use eh3d_printmaterials::Lang;
pub use error::{PrnCode, PrnError};
pub use i18n::Label;
pub use model::{
    Adapter, Enclosure, Firmware, Multicolor, MulticolorStatus, Printer, PrinterFile, SendFormat,
    Source, SCHEMA_VERSION,
};
pub use start::{starting_filaments, Block, Caution, Level, Process, Processes, StartFilament};
