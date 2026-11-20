//! # eh3d-printmaterials
//!
//! The database of 3D printing materials of the suite: FDM filaments and photopolymer resins
//! (SLA, DLP, LCD, MSLA), each with the values its maker publishes. The slicer reads the whole
//! record (temperatures, flow, drying, exposure by printer and layer); any other program can read
//! the same data and ask the same questions.
//!
//! * [`model`]: the records, as written in the JSON files (a field the schema does not know is
//!   refused, a value the sheet does not publish is `null`, nothing is marked verified, and
//!   there is no price: the price is the user's);
//! * [`data`]: the data files, embedded in the crate, one per manufacturer;
//! * [`validate`]: the rules the data follows (unique identifiers that agree with the
//!   manufacturer and the name, believable temperatures, flows and times, ranges in order, a
//!   source and `https` links);
//! * [`query`]: the [`Database`], the lookups by identifier and the searches by material, nozzle,
//!   bed, filler, diameter, printer and kind of resin;
//! * [`calc`]: what a slicer asks of a record (`nozzle_range`, `bed_range`, `mm3s_limit`,
//!   `grams_per_meter`, `drying_plan`, `needs_hardened_nozzle`, `exposure_for`), the cost per
//!   gram and per metre from a price the caller gives, and the exposure of a resin for a
//!   printer and a layer height, interpolated between known points and never extrapolated;
//! * [`i18n`]: the titles of the error codes, the warnings and the notes in the eight languages
//!   of the suite;
//! * [`error`]: the typed errors with the stable codes `EH3D-PMAT-001` to `EH3D-PMAT-009`.
//!
//! Nothing here opens a window, a file or the network: the data is part of the library.

#![forbid(unsafe_code)]

pub mod calc;
pub mod data;
pub mod error;
pub mod i18n;
pub mod model;
pub mod query;
pub mod validate;

pub use calc::{
    cost_per_gram, cost_per_meter, grams_for_volume, grams_per_meter, meters_for_grams, DryingPlan,
    Exposure,
};
pub use error::{PmCode, PmError};
pub use i18n::{ExposureNote, ExposureWarning, Lang};
pub use model::{
    ExposurePoint, Filament, FilamentFile, Fill, Material, Resin, ResinFile, ResinKind,
    SCHEMA_VERSION,
};
pub use query::{parse_filament_file, parse_resin_file, Database, FilamentCriteria, ResinCriteria};
