//! # eh3d-container
//!
//! The pieces that every project container of the suite shares, taken out of the `.eh3dproj`
//! code so that another format can use them:
//!
//! * [`spec`]: the parameters that tell one container format from another (roots, limits,
//!   budget, JSON form, entry order, supported versions, name policy) and the presets
//!   (`.eh3dproj` 1.0.0 exactly as it behaves today, `.eh3dproj` 1.1.0 and `.phoenixproj`);
//! * [`integrity`]: the integrity record (SHA-256 and size of every entry), its verification and
//!   the content hash that ignores variable audit data;
//! * [`canonical`]: canonical JSON (RFC 8785) for every new JSON entry;
//! * [`codes`] and [`error`]: the stable codes `EH3D-PKG-001` to `EH3D-PKG-014`, and the
//!   diagnostics built from them;
//! * [`policy`]: the name policy (the rule of 1.0.0, or the strict one for new formats);
//! * [`reader`]: the bounded, streaming reader (`Container`);
//! * [`writer`] and [`atomic`]: the streaming, deterministic writer and the atomic replacement
//!   of the target, with bounded retries where another program may hold the file.
//!
//! The `.eh3dproj` 1.0.0 code of `eh3d-common` still reads and writes its packages itself and
//! keeps its behaviour; the new reader and writer serve the strict policy of new formats (the
//! legacy preset reproduces the 1.0.0 bytes when written, which a test checks).
//!
//! Nothing here opens a window or the network.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod atomic;
pub mod canonical;
pub mod codes;
pub mod error;
pub mod integrity;
pub mod policy;
pub mod reader;
pub mod spec;
pub mod writer;

pub use atomic::{clean_orphan_temporaries, write_atomic, AtomicOptions};
pub use canonical::{
    canonical_file_bytes, canonical_string, to_canonical_file_bytes, CanonicalError,
};
pub use codes::PkgCode;
pub use error::{violation_in, ContainerError, Violation};
pub use integrity::{sha256_hex, Integrity, IntegrityEntry, IntegrityError};
pub use policy::{check_name, collision_key, is_windows_reserved, Policy};
pub use reader::{Container, EntryInfo, EntryReader};
pub use spec::{
    check_entry_name, Budget, ContainerSpec, EntryOrder, JsonForm, Limits, Method, NameError,
    NewerMinor,
};
pub use writer::ContainerWriter;
