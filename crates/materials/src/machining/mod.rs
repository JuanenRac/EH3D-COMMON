//! The second family of the database: the materials that a CNC router or a laser cuts, engraves
//! or mills.
//!
//! The board database (the rest of the crate) describes laminates for the design tool. This family
//! describes workpieces for the machine programs: woods, acrylic, paper, leather, rubber, fabric,
//! foam, engineering plastics, metals and board stock. For each one it says
//!
//! * what it is (identifier `<family>/<name>`, the name in eight languages, density, commercial
//!   thicknesses);
//! * what a laser may do with it and why not (`laser_cut_allowed`, `laser_engrave_allowed`,
//!   `laser_unsafe` and the hazards, whose texts are in eight languages): PVC, ABS, polycarbonate,
//!   vinyl, carbon fibre and the like are refused;
//! * starting points for a CNC router (cutter, diameter, spindle speed, feed, depth of a pass) and
//!   for a laser (kind, nominal power, thickness, power setting, speed, passes).
//!
//! Every starting point carries its source and says whether anybody tried it on a machine; the
//! seed of the crate never was. A point is only a place to start. The queries
//! ([`Workpiece::cnc_start`] and [`Workpiece::laser_start`]) give the point itself, or a straight
//! line between two known points, and never go outside what is known: they answer with a
//! warning, in eight languages, instead.
//!
//! The family keeps its own files (`data/machining/`) and its own explicit list of them
//! ([`data::FILES`]); the board database is not touched by it. A workpiece that is board stock
//! may name the laminate of the board database it is with `pcb_material_id`
//! ([`MachiningDatabase::check_links`]).

pub mod data;
pub mod model;
pub mod query;
pub mod start;
pub mod texts;
pub mod validate;

pub use model::{
    CncOp, CncPoint, Family, Hazard, LaserKind, LaserMode, LaserPoint, LaserProcess, MachiningFile,
    Names, Safety, Tool, Workpiece,
};
pub use query::MachiningDatabase;
pub use start::{CncStart, LaserStart, Quantity, StartNote, StartWarning};
