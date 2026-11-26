//! # eh3d-project
//!
//! The `.eh3d` project container of the suite: one file, one **branch** for each program
//! (`pcb/`, `cad/`, `3dp/`, `laser/`, `cnc/`, ...), a `common/` branch for the project as a
//! whole, a `manufacturing/` branch that is a `.eh3dproj` 1.0.0 package, `docs/<program>/`
//! branches for documentation and `x-*/` folders for extensions. It is built on the shared
//! `eh3d-container` (strict ZIP, SHA-256 of every entry, deterministic and atomic writing,
//! stable `EH3D-PKG-NNN` codes) and described in `docs/FORMAT_EH3D.md`.
//!
//! * [`spec`]: the presets (`eh3d`, `manufacturing`) and the layout rules of the folders;
//! * [`records`]: `project.json`, `branch.json`, references between branches with the hash
//!   and the version they were taken at, the content hash of a branch;
//! * [`meta`]: the optional metadata, the index of documents (in the eight languages of the
//!   project) and the report of a conversion;
//! * [`security`]: the traits of signing and checking and the status of a signature;
//! * [`ed25519`]: the signature algorithm of the suite and its key files, which live outside
//!   every project;
//! * [`appbranch`]: what a program does with its own branch (save, open, import with a report,
//!   take from another branch, out-of-date notices), so that each program does not write it
//!   again;
//! * [`manufacturing`]: how the programs that make something from the job of the project read it and keep their result;
//! * [`attachments`]: any file attached to the project (data sheets, manuals, other projects), linked to the
//!   pieces of the design, inside it or only as an address with its hash, with the dangerous ones marked, and dangerous or unknown ones
//!   taken out only when a person agrees;
//! * [`suiteview`] and [`suitetext`]: what a window shows and asks before it writes a project, and its words in the eight
//!   languages;
//! * [`project`]: open, read the branches of other programs, check, save one branch without
//!   touching the bytes of the others, find derived branches that are out of date, and take the
//!   `manufacturing` branch in and out as a `.eh3dproj`.
//!
//! What is designed and has its hooks but is not implemented yet: encryption of a branch
//! (a branch that says it is encrypted is refused for reading with `EH3D-PKG-023` and kept as it
//! is). Nothing here opens a window or the network, reads the clock or runs anything that is in
//! a file. The only program ever started is the system tool `icacls`, on Windows, to limit a new
//! secret key file to its owner.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod appbranch;
pub mod attachments;
pub mod ed25519;
pub mod manufacturing;
pub mod meta;
pub mod project;
pub mod records;
pub mod security;
pub mod spec;
pub mod suitetext;
pub mod suiteview;

pub use attachments::{
    AttachError, AttachRequest, Attacher, Attachment, AttachmentIndex, AttachmentLimits,
    AttachmentSource, ComponentLink, ComponentQuery, Consent,
};
pub use eh3d_filekind::{AttachmentKind, Danger};
pub use ed25519::{Ed25519Keyring, Ed25519Signer, KeyError, KeyPair};
pub use project::{
    manufacturing_update, Branch, BranchUpdate, Checks, FileInfo, Finding, NewProject, OpenOptions,
    Project, SaveOptions, StaleRef, Writer, AUDIT_LOG,
};
pub use records::{
    branch_content_hash, BranchIndexEntry, BranchRecord, Dependency, EncryptionRecord,
    ProjectRecord, Seal, SignatureRecord,
};
pub use security::{SignatureReport, SignatureStatus, Signer, Verifier, VerifyOutcome};
