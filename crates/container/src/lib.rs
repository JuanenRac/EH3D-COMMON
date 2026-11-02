//! # eh3d-container
//!
//! The pieces that every project container of the suite shares, taken out of the `.eh3dproj`
//! code so that another format can use them:
//!
//! * [`spec`]: the parameters that tell one container format from another (roots, limits,
//!   JSON form, entry order, supported versions) and the preset that describes `.eh3dproj` 1.0.0
//!   exactly as it behaves today;
//! * [`integrity`]: the integrity record (SHA-256 and size of every entry), its verification and
//!   the content hash that ignores variable audit data;
//! * [`canonical`]: canonical JSON (RFC 8785) for every new JSON entry.
//!
//! What is not here yet: the ZIP reader and writer themselves (they stay in `eh3d-common` and
//! keep their behaviour), the stricter name and ZIP policy for new presets and the numbered
//! diagnostic codes of the container; each waits for an open decision.
//!
//! Nothing here opens a file, a window or the network.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod canonical;
pub mod integrity;
pub mod spec;

pub use canonical::{
    canonical_file_bytes, canonical_string, to_canonical_file_bytes, CanonicalError,
};
pub use integrity::{sha256_hex, Integrity, IntegrityEntry, IntegrityError};
pub use spec::{
    check_entry_name, ContainerSpec, EntryOrder, JsonForm, Limits, Method, NameError, NewerMinor,
};
