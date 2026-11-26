//! Opening, reading, checking and saving a `.eh3d` project.
//!
//! The rules this module keeps (see `docs/FORMAT_EH3D.md`):
//!
//! * a program writes only its own branch (and the shared `common` and `docs/project`); saving a
//!   branch copies every other entry through unchanged, so their bytes do not move;
//! * a branch this program does not know is kept exactly, never read and never rewritten;
//! * a branch that was derived from another one says from which state, and a derived branch
//!   whose source has changed is reported as out of date (a warning, never a refusal);
//! * the file is replaced atomically, and the same content gives the same bytes: nothing reads
//!   the clock, the caller says what time to write.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use eh3d_container::{
    check_entry_name, to_canonical_file_bytes, AtomicOptions, Container, ContainerError,
    ContainerSpec, ContainerWriter, EntryReader, Integrity, PkgCode, Violation,
};
use eh3d_diagnostic::Diagnostic;
use serde_json::Value;

use crate::attachments::{AttachmentIndex, INDEX_FILE};
use crate::meta::{ConversionReport, DocIndex, Metadata, DOC_INDEX_FILE, META_FILE, REPORT_FILE};
use crate::records::{
    branch_content_hash, try_signing_message, BranchIndexEntry, BranchRecord, Dependency,
    EncryptionRecord, ProjectRecord, Seal, SignatureRecord, BRANCH_SIGNATURE_LABEL,
    PROJECT_SIGNATURE_LABEL,
};
use crate::security::{sign_with, status_of, SignatureReport, SignatureStatus, Signer, Verifier};
use crate::spec::{
    branch_dir, classify, eh3d, entry_name, is_sha256_hex, is_shared_branch, manufacturing,
    parse_major_minor, parse_version, root_of, valid_branch_id, EntryClass, ATTACHMENTS_BRANCH,
    BRANCH_FILE, COMMON_BRANCH, CONTAINER_VERSION, DOCS_ROOT, FORMAT_VERSION, MANUFACTURING_BRANCH,
    RECORD_SCHEMA,
};

/// Everything here fails with the error of the container: a refusal with a stable code, or a
/// failure of the file system.
pub type Result<T> = std::result::Result<T, ContainerError>;

/// The largest record (`project.json`, `branch.json`, an index) that is read into memory.
const MAX_RECORD: u64 = 16 * 1024 * 1024;
/// The largest audit log that is read and extended.
const MAX_AUDIT: u64 = 64 * 1024 * 1024;
/// The name of the audit log.
pub const AUDIT_LOG: &str = "audit/log.jsonl";

fn refuse(code: PkgCode, detail: impl Into<String>) -> ContainerError {
    ContainerError::Rejected(Violation::new(code, detail))
}

fn refuse_at(code: PkgCode, detail: impl Into<String>, entry: &str) -> ContainerError {
    ContainerError::Rejected(Violation::new(code, detail).at(entry))
}

/// The size and the SHA-256 of one file of a branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// The size in bytes.
    pub size: u64,
    /// The SHA-256 in lowercase hexadecimal.
    pub sha256: String,
}

/// One branch of an open project.
#[derive(Debug, Clone)]
pub struct Branch {
    /// The name, for example `pcb` or `docs/pcb`.
    pub id: String,
    /// The files, by the name inside the branch (`branch.json` included).
    pub files: BTreeMap<String, FileInfo>,
    /// The record, when `branch.json` is there and can be read.
    pub record: Option<BranchRecord>,
    /// Why there is no record.
    pub record_problem: Option<String>,
    /// The content hash worked out from the files.
    pub hash: String,
}

impl Branch {
    /// True for an extension folder (`x-...`): kept and never interpreted, it needs no record.
    pub fn is_extension(&self) -> bool {
        self.id.starts_with("x-")
    }

    /// True when this crate knows the branch by name.
    pub fn is_known(&self) -> bool {
        let root = self.id.split('/').next().unwrap_or("");
        crate::spec::KNOWN_BRANCHES.contains(&root)
    }

    /// True when the branch says it is encrypted.
    pub fn is_encrypted(&self) -> bool {
        self.record.as_ref().is_some_and(|r| r.encryption.is_some())
    }

    /// True when the branch says it is sealed.
    pub fn is_sealed(&self) -> bool {
        self.record.as_ref().is_some_and(|r| r.seal.is_some())
    }
}

/// How a project is opened.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// What the container allows (limits, budget); the default is [`eh3d`].
    pub spec: ContainerSpec,
    /// Whether every entry is read and hashed against `integrity.json` (the default). Without it
    /// the hashes are those the record claims and nothing is proved.
    pub verify_integrity: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions {
            spec: eh3d(),
            verify_integrity: true,
        }
    }
}

/// What a program says about itself when it writes: its name and the branches it owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Writer {
    /// The program, as it is written in the `app` field of its branches.
    pub app: String,
    /// The branches it may write besides the shared ones.
    pub branches: BTreeSet<String>,
}

impl Writer {
    /// A writer that owns the given branches.
    pub fn new(app: &str, branches: &[&str]) -> Writer {
        Writer {
            app: app.to_owned(),
            branches: branches.iter().map(|b| (*b).to_owned()).collect(),
        }
    }

    /// A program that owns the branch `id` and its documentation `docs/<id>`.
    pub fn for_app(id: &str) -> Writer {
        Writer::new(id, &[id, &format!("{DOCS_ROOT}/{id}")])
    }

    /// True when this writer owns `id`.
    pub fn claims(&self, id: &str) -> bool {
        self.branches.contains(id)
    }
}

/// What a program hands over to save one branch.
#[derive(Debug, Clone)]
pub struct BranchUpdate {
    /// The branch name.
    pub id: String,
    /// The format version of the branch, `major.minor`.
    pub format: String,
    /// The version of the program that writes it.
    pub app_version: String,
    /// The whole new content: names inside the branch and their bytes (not `branch.json`).
    pub entries: BTreeMap<String, Vec<u8>>,
    /// The branches it was derived from, with the state they were in.
    pub depends: Vec<Dependency>,
    /// A seal to put on the branch with this write.
    pub seal: Option<Seal>,
    /// The encryption record, when the bytes in `entries` are already ciphertext.
    pub encryption: Option<EncryptionRecord>,
    /// Fields of the record that this crate does not know.
    pub extra: BTreeMap<String, Value>,
}

impl BranchUpdate {
    /// An update with no entries yet.
    pub fn new(id: &str, format: &str, app_version: &str) -> BranchUpdate {
        BranchUpdate {
            id: id.to_owned(),
            format: format.to_owned(),
            app_version: app_version.to_owned(),
            entries: BTreeMap::new(),
            depends: Vec::new(),
            seal: None,
            encryption: None,
            extra: BTreeMap::new(),
        }
    }

    /// Adds an entry.
    #[must_use]
    pub fn with_entry(mut self, rel: &str, bytes: &[u8]) -> BranchUpdate {
        self.entries.insert(rel.to_owned(), bytes.to_vec());
        self
    }
}

/// A file of a branch that is written from a part of a file on disk, without holding it in memory. The size and the hash are the
/// ones the branch record states, so a file that changes while it is read is refused before anything is replaced.
#[derive(Debug, Clone)]
pub(crate) struct StreamedFile {
    pub(crate) rel: String,
    pub(crate) path: PathBuf,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

/// A write of a branch whose files are not all in memory (used by the attachments): what is kept as it is, what is new from memory
/// and what is new from disk. Every other file of the branch is dropped.
#[derive(Debug, Default)]
pub(crate) struct StreamedUpdate {
    pub(crate) id: String,
    pub(crate) format: String,
    pub(crate) app_version: String,
    pub(crate) keep: BTreeSet<String>,
    pub(crate) entries: BTreeMap<String, Vec<u8>>,
    pub(crate) files: Vec<StreamedFile>,
    pub(crate) extra: BTreeMap<String, Value>,
}

/// The settings of a save.
#[derive(Default)]
pub struct SaveOptions<'a> {
    /// The time to write in `modified` (the caller's clock); `None` keeps the old value.
    pub modified: Option<String>,
    /// When set, a line is added to the audit log with this time.
    pub audit_at: Option<String>,
    /// How the file is replaced.
    pub atomic: AtomicOptions,
    /// Signs the branch that is saved.
    pub signer: Option<&'a dyn Signer>,
}

/// The data of a new project.
#[derive(Debug, Clone)]
pub struct NewProject {
    /// The identifier (a UUID is the usual choice).
    pub id: String,
    /// The name.
    pub name: String,
    /// The unit of lengths.
    pub units: String,
    /// When it was created, as the caller writes it.
    pub created: Option<String>,
}

/// One thing a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code.
    pub code: PkgCode,
    /// What exactly, in English.
    pub detail: String,
    /// The branch it concerns.
    pub branch: Option<String>,
    /// The entry it concerns.
    pub entry: Option<String>,
}

impl Finding {
    /// True when the finding is an error (every code except the warning of 019).
    pub fn is_error(&self) -> bool {
        !self.code.is_warning()
    }

    /// The diagnostic of the finding.
    pub fn diagnostic(&self) -> Diagnostic {
        self.code.diagnostic(&self.detail, self.entry.as_deref())
    }
}

/// What a program asks of [`Project::diagnose`].
#[derive(Default)]
pub struct Checks<'a> {
    /// The newest `(major, minor)` format of each branch this program can handle.
    pub supported_formats: BTreeMap<String, (u64, u64)>,
    /// The branches that must carry a valid signature.
    pub require_signed: BTreeSet<String>,
    /// Whether the project itself must be signed.
    pub require_project_signature: bool,
    /// Checks signatures; without it signatures are not judged.
    pub verifier: Option<&'a dyn Verifier>,
}

/// A derived branch whose source has changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleRef {
    /// The branch that holds the reference.
    pub branch: String,
    /// The reference, as it was written.
    pub dependency: Dependency,
    /// The content hash of the source now.
    pub current_hash: String,
}

/// An open project.
pub struct Project {
    path: PathBuf,
    container: Container<File>,
    spec: ContainerSpec,
    verified: bool,
    record: ProjectRecord,
    branches: BTreeMap<String, Branch>,
    audit: Vec<String>,
    integrity: Integrity,
}

fn parse_json<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    what: &str,
) -> std::result::Result<T, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("{what} is not a valid record: {e}"))
}

impl Project {
    /// Opens the project at `path`: the structure of the ZIP, the names, the limits, the format
    /// version (before anything else), the layout and, unless switched off, every entry against
    /// `integrity.json`. Problems of the records of the branches are not refusals: they are
    /// listed by [`Project::diagnose`], so a program can still open the file, write its own
    /// branch and keep the rest.
    pub fn open(path: &Path, options: &OpenOptions) -> Result<Project> {
        let spec = options.spec.clone();
        let mut container = Container::open_path(path, &spec)?;
        for required in &spec.required_entries {
            if container.entry(required).is_none() {
                let code = if required == "integrity.json" {
                    PkgCode::Integrity
                } else {
                    PkgCode::OutsideRoots
                };
                return Err(refuse_at(code, "a required entry is missing", required));
            }
        }
        let raw = container.read_limited("project.json", MAX_RECORD)?;
        let value: Value = serde_json::from_slice(&raw).map_err(|_| {
            refuse_at(
                PkgCode::BranchRecord,
                "project.json is not valid JSON",
                "project.json",
            )
        })?;
        let version = value
            .get("format_version")
            .and_then(Value::as_str)
            .and_then(parse_version)
            .ok_or_else(|| {
                refuse_at(
                    PkgCode::BranchRecord,
                    "project.json has no readable format_version",
                    "project.json",
                )
            })?;
        if !spec.supports_version(version.0, version.1) {
            return Err(refuse_at(
                PkgCode::NewerVersion,
                format!(
                    "version {}.{} is not supported by this reader ({}.{})",
                    version.0, version.1, spec.major_version, spec.known_minor
                ),
                "project.json",
            ));
        }
        let record: ProjectRecord = serde_json::from_value(value).map_err(|e| {
            refuse_at(
                PkgCode::BranchRecord,
                format!("project.json: {e}"),
                "project.json",
            )
        })?;
        if record.schema > RECORD_SCHEMA {
            return Err(refuse_at(
                PkgCode::NewerVersion,
                "the project record has a newer schema",
                "project.json",
            ));
        }
        if parse_major_minor(&record.container_version).is_none_or(|(major, _)| major != 1) {
            return Err(refuse_at(
                PkgCode::NewerVersion,
                "the container rules are not of version 1",
                "project.json",
            ));
        }

        let names: Vec<String> = container.entries().map(|e| e.name.clone()).collect();
        for name in &names {
            if classify(name).is_none() {
                return Err(refuse_at(
                    PkgCode::OutsideRoots,
                    "the entry has no place in the layout of a project",
                    name,
                ));
            }
        }
        let integrity = if options.verify_integrity {
            container.verify_integrity()?
        } else {
            let bytes = container.read_limited("integrity.json", MAX_RECORD)?;
            serde_json::from_slice::<Integrity>(&bytes).map_err(|_| {
                refuse_at(
                    PkgCode::Integrity,
                    "integrity.json is not a valid record",
                    "integrity.json",
                )
            })?
        };

        let mut grouped: BTreeMap<String, BTreeMap<String, FileInfo>> = BTreeMap::new();
        let mut audit = Vec::new();
        for name in &names {
            let Some(class) = classify(name) else {
                continue;
            };
            match class {
                EntryClass::Project | EntryClass::Integrity => {}
                EntryClass::Audit => audit.push(name.clone()),
                EntryClass::Branch { id, rel } => {
                    let Some(claimed) = integrity.files.get(name) else {
                        return Err(refuse_at(
                            PkgCode::Integrity,
                            "the entry is not listed in the record",
                            name,
                        ));
                    };
                    grouped.entry(id.to_owned()).or_default().insert(
                        rel.to_owned(),
                        FileInfo {
                            size: claimed.size,
                            sha256: claimed.sha256.clone(),
                        },
                    );
                }
            }
        }
        let mut branches = BTreeMap::new();
        for (id, files) in grouped {
            let hash = branch_content_hash(
                files
                    .iter()
                    .map(|(n, f)| (n.as_str(), f.size, f.sha256.as_str())),
            );
            let (record, problem) = if files.contains_key(BRANCH_FILE) {
                let bytes = container.read_limited(&entry_name(&id, BRANCH_FILE), MAX_RECORD)?;
                match parse_json::<BranchRecord>(&bytes, "branch.json") {
                    Ok(r) => (Some(r), None),
                    Err(e) => (None, Some(e)),
                }
            } else {
                (None, Some("branch.json is missing".to_owned()))
            };
            branches.insert(
                id.clone(),
                Branch {
                    id,
                    files,
                    record,
                    record_problem: problem,
                    hash,
                },
            );
        }
        Ok(Project {
            path: path.to_owned(),
            container,
            spec,
            verified: options.verify_integrity,
            record,
            branches,
            audit,
            integrity,
        })
    }

    /// Creates a project with no branches. An existing file is never replaced.
    pub fn create(
        path: &Path,
        new: NewProject,
        options: &OpenOptions,
        atomic: &AtomicOptions,
    ) -> Result<Project> {
        if path.exists() {
            return Err(ContainerError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "the file already exists and is not replaced",
            )));
        }
        if new.id.trim().is_empty() || new.name.trim().is_empty() || new.id.len() > 128 {
            return Err(refuse_at(
                PkgCode::BranchRecord,
                "a project needs an identifier and a name",
                "project.json",
            ));
        }
        let record = ProjectRecord {
            schema: RECORD_SCHEMA,
            format_version: FORMAT_VERSION.to_owned(),
            container_version: CONTAINER_VERSION.to_owned(),
            id: new.id,
            name: new.name,
            units: new.units,
            created: new.created.clone(),
            modified: new.created,
            branches: Vec::new(),
            seal: None,
            signature: None,
            extra: BTreeMap::new(),
        };
        assemble(None, path, &options.spec, Plan::default(), &record, atomic)?;
        Project::open(path, options)
    }

    /// The limits the project was opened with.
    pub(crate) fn spec(&self) -> &ContainerSpec {
        &self.spec
    }

    /// The file the project was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The project record.
    pub fn record(&self) -> &ProjectRecord {
        &self.record
    }

    /// The branches, by name.
    pub fn branches(&self) -> impl Iterator<Item = &Branch> {
        self.branches.values()
    }

    /// One branch.
    pub fn branch(&self, id: &str) -> Option<&Branch> {
        self.branches.get(id)
    }

    /// The files of the audit log.
    pub fn audit_entries(&self) -> &[String] {
        &self.audit
    }

    /// True when the integrity record was checked against every entry when the project opened.
    pub fn integrity_verified(&self) -> bool {
        self.verified
    }

    /// The content hash of the whole container (the audit entries left out).
    pub fn content_hash(&self) -> Result<String> {
        self.integrity
            .content_hash()
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))
    }

    /// Reads a file of any branch, whoever wrote it (reading is allowed to every program). An
    /// encrypted branch is refused (023) except for its `branch.json`.
    pub fn read_entry(&mut self, branch: &str, rel: &str) -> Result<Vec<u8>> {
        let name = self.checked_name(branch, rel)?;
        let limit = self.spec.limits.max_entry_bytes_for(&name);
        self.container.read_limited(&name, limit)
    }

    /// A bounded stream of a file of any branch; see [`Project::read_entry`].
    pub fn read_stream(&mut self, branch: &str, rel: &str) -> Result<EntryReader<'_, File>> {
        let name = self.checked_name(branch, rel)?;
        self.container.read_stream(&name)
    }

    fn checked_name(&self, branch: &str, rel: &str) -> Result<String> {
        let found = self
            .branches
            .get(branch)
            .ok_or_else(|| refuse_at(PkgCode::BranchRef, "there is no such branch", branch))?;
        if !found.files.contains_key(rel) {
            return Err(refuse_at(
                PkgCode::OutsideRoots,
                "there is no such file in the branch",
                &entry_name(branch, rel),
            ));
        }
        if rel != BRANCH_FILE && found.is_encrypted() {
            return Err(refuse_at(
                PkgCode::EncryptedBranch,
                "the branch is encrypted and no key was given",
                branch,
            ));
        }
        Ok(entry_name(branch, rel))
    }

    /// The derived branches whose sources have changed since they were derived.
    pub fn stale_refs(&self) -> Vec<StaleRef> {
        let mut out = Vec::new();
        for branch in self.branches.values() {
            let Some(record) = &branch.record else {
                continue;
            };
            for dep in &record.depends {
                if let Some(source) = self.branches.get(&dep.branch) {
                    if source.hash != dep.hash {
                        out.push(StaleRef {
                            branch: branch.id.clone(),
                            dependency: dep.clone(),
                            current_hash: source.hash.clone(),
                        });
                    }
                }
            }
        }
        out
    }

    /// A reference to the present state of `source`, ready to be written into the branch that
    /// is derived from it (this is how an out-of-date derivative is brought up to date: derive
    /// again, then save the branch with this reference).
    pub fn dependency_on(
        &self,
        source: &str,
        artifact: &str,
        artifact_version: u32,
        uses: Vec<String>,
    ) -> Result<Dependency> {
        let found = self.branches.get(source).ok_or_else(|| {
            refuse_at(
                PkgCode::BranchRef,
                "the source branch is not in this file",
                source,
            )
        })?;
        let record = found.record.as_ref().ok_or_else(|| {
            refuse_at(
                PkgCode::BranchRecord,
                "the source branch has no valid record",
                source,
            )
        })?;
        Ok(Dependency {
            branch: source.to_owned(),
            hash: found.hash.clone(),
            format: record.format.clone(),
            artifact: artifact.to_owned(),
            artifact_version,
            uses,
            extra: BTreeMap::new(),
        })
    }

    /// Checks the records of the project and of every branch and lists what is wrong. A
    /// project with no error findings is sound; warnings (out-of-date derivatives) do not make
    /// it unsound.
    pub fn diagnose(&mut self, checks: &Checks<'_>) -> Vec<Finding> {
        let mut out = Vec::new();
        let mut add =
            |code: PkgCode, detail: String, branch: Option<&str>, entry: Option<String>| {
                out.push(Finding {
                    code,
                    detail,
                    branch: branch.map(str::to_owned),
                    entry,
                });
            };

        // The index in project.json against what is in the file.
        let indexed: BTreeMap<&str, &BranchIndexEntry> = self
            .record
            .branches
            .iter()
            .map(|b| (b.id.as_str(), b))
            .collect();
        for branch in self.branches.values().filter(|b| !b.is_extension()) {
            let Some(entry) = indexed.get(branch.id.as_str()) else {
                add(
                    PkgCode::BranchRecord,
                    "the branch is not in the index of project.json".into(),
                    Some(branch.id.as_str()),
                    Some("project.json".into()),
                );
                continue;
            };
            if entry.content_hash != branch.hash {
                add(
                    PkgCode::BranchHash,
                    "the index of project.json states another content hash".into(),
                    Some(branch.id.as_str()),
                    Some("project.json".into()),
                );
            }
            if let Some(record) = &branch.record {
                if (&record.app, &record.format, &record.app_version)
                    != (&entry.app, &entry.format, &entry.app_version)
                {
                    add(
                        PkgCode::BranchRecord,
                        "the index of project.json and branch.json disagree".into(),
                        Some(branch.id.as_str()),
                        Some("project.json".into()),
                    );
                }
            }
        }
        for id in indexed.keys() {
            if !self.branches.contains_key(*id) {
                add(
                    PkgCode::BranchRecord,
                    "the index lists a branch that is not in the file".into(),
                    Some(*id),
                    Some("project.json".into()),
                );
            }
        }

        let ids: Vec<String> = self.branches.keys().cloned().collect();
        for id in ids.iter().map(String::as_str) {
            let branch = self.branches[id].clone();
            if branch.is_extension() {
                continue;
            }
            let record_entry = entry_name(id, BRANCH_FILE);
            let Some(record) = &branch.record else {
                let why = branch.record_problem.clone().unwrap_or_default();
                add(PkgCode::BranchRecord, why, Some(id), Some(record_entry));
                continue;
            };
            let here = |detail: &str| detail.to_owned();
            if record.schema > RECORD_SCHEMA {
                add(
                    PkgCode::BranchVersion,
                    here("the record has a newer schema"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            } else if record.schema == 0 {
                add(
                    PkgCode::BranchRecord,
                    here("the schema of the record is 0"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            }
            if record.branch != id {
                add(
                    PkgCode::BranchRecord,
                    here("the record names another branch than its folder"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            }
            if record.app.trim().is_empty() || record.app_version.trim().is_empty() {
                add(
                    PkgCode::BranchRecord,
                    here("the record has no program or no program version"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            }
            match parse_major_minor(&record.format) {
                None => add(
                    PkgCode::BranchRecord,
                    here("the format of the branch is not major.minor"),
                    Some(id),
                    Some(record_entry.clone()),
                ),
                Some(found) => {
                    if let Some(supported) = checks.supported_formats.get(id) {
                        if found.0 != supported.0 || found.1 > supported.1 {
                            add(
                                PkgCode::BranchVersion,
                                format!(
                                    "format {}.{} is not supported ({}.{})",
                                    found.0, found.1, supported.0, supported.1
                                ),
                                Some(id),
                                Some(record_entry.clone()),
                            );
                        }
                    }
                }
            }
            if !is_sha256_hex(&record.content_hash) {
                add(
                    PkgCode::BranchRecord,
                    here("the content hash is not a SHA-256"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            } else if record.content_hash != branch.hash {
                add(
                    PkgCode::BranchHash,
                    here("the files of the branch are not the ones its record hashes"),
                    Some(id),
                    Some(record_entry.clone()),
                );
            }

            for dep in &record.depends {
                let bad = |why: &str| format!("a reference to '{}': {why}", dep.branch);
                if !valid_branch_id(&dep.branch) || dep.branch == id {
                    add(
                        PkgCode::BranchRef,
                        bad("it does not name another branch of this file"),
                        Some(id),
                        Some(record_entry.clone()),
                    );
                } else if !is_sha256_hex(&dep.hash)
                    || parse_major_minor(&dep.format).is_none()
                    || dep.artifact.is_empty()
                {
                    add(
                        PkgCode::BranchRef,
                        bad("its hash, format or artifact is malformed"),
                        Some(id),
                        Some(record_entry.clone()),
                    );
                } else if let Some(source) = self.branches.get(&dep.branch) {
                    if source.hash != dep.hash {
                        add(
                            PkgCode::StaleDerived,
                            format!(
                                "derived from '{}' as it was before it changed ({})",
                                dep.branch, dep.artifact
                            ),
                            Some(id),
                            Some(record_entry.clone()),
                        );
                    }
                } else {
                    add(
                        PkgCode::BranchRef,
                        bad("that branch is not in this file"),
                        Some(id),
                        Some(record_entry.clone()),
                    );
                }
            }

            // The optional records, when they are there and can be read.
            if !branch.is_encrypted() {
                self.check_optional(&branch, &mut add);
            }

            let status = self.status_of_branch(&branch, checks.verifier);
            judge_signature(
                &status,
                checks.require_signed.contains(id),
                Some(id),
                &record_entry,
                &mut add,
            );
        }

        let status = self.project_signature(checks.verifier);
        judge_signature(
            &status,
            checks.require_project_signature,
            None,
            "project.json",
            &mut add,
        );
        out
    }

    fn check_optional(
        &mut self,
        branch: &Branch,
        add: &mut impl FnMut(PkgCode, String, Option<&str>, Option<String>),
    ) {
        let id = branch.id.as_str();
        let files = || {
            branch
                .files
                .iter()
                .map(|(n, f)| (n.as_str(), f.size, f.sha256.as_str()))
        };
        if id == COMMON_BRANCH && branch.files.contains_key(META_FILE) {
            let name = entry_name(id, META_FILE);
            let outcome = self
                .read_record::<Metadata>(&name)
                .and_then(|m| m.validate());
            if let Err(why) = outcome {
                add(PkgCode::BranchRecord, why, Some(id), Some(name));
            }
        }
        if id.starts_with("docs/") && branch.files.contains_key(DOC_INDEX_FILE) {
            let name = entry_name(id, DOC_INDEX_FILE);
            let outcome = self
                .read_record::<DocIndex>(&name)
                .and_then(|index| index.validate(branch.files.keys().map(String::as_str)));
            if let Err(why) = outcome {
                add(PkgCode::BranchRecord, why, Some(id), Some(name));
            }
        }
        if id == ATTACHMENTS_BRANCH {
            let name = entry_name(id, INDEX_FILE);
            let outcome = if branch.files.contains_key(INDEX_FILE) {
                self.read_record::<AttachmentIndex>(&name)
                    .and_then(|index| index.validate(files()))
            } else {
                Err("the index of the attachments is missing".to_owned())
            };
            if let Err(why) = outcome {
                add(PkgCode::BranchRecord, why, Some(id), Some(name));
            }
        }
        if branch.files.contains_key(REPORT_FILE) {
            let name = entry_name(id, REPORT_FILE);
            let outcome = self
                .read_record::<ConversionReport>(&name)
                .and_then(|report| report.validate(files()));
            if let Err(why) = outcome {
                add(PkgCode::BranchRecord, why, Some(id), Some(name));
            }
        }
    }

    fn read_record<T: serde::de::DeserializeOwned>(
        &mut self,
        name: &str,
    ) -> std::result::Result<T, String> {
        let bytes = self
            .container
            .read_limited(name, MAX_RECORD)
            .map_err(|e| e.to_string())?;
        parse_json(&bytes, name)
    }

    /// Opens, checks and reports: a project is sound when no finding is an error.
    pub fn verify(
        path: &Path,
        options: &OpenOptions,
        checks: &Checks<'_>,
    ) -> Result<(Project, Vec<Finding>)> {
        let mut project = Project::open(path, options)?;
        let findings = project.diagnose(checks);
        Ok((project, findings))
    }

    /// Opens the project (integrity always checked) and judges every signature it carries with
    /// the keys of `verifier`. A wrong signature does not stop the opening: the report says so
    /// and the program decides.
    pub fn open_verified(
        path: &Path,
        options: &OpenOptions,
        verifier: Option<&dyn Verifier>,
    ) -> Result<(Project, SignatureReport)> {
        let options = OpenOptions {
            spec: options.spec.clone(),
            verify_integrity: true,
        };
        let project = Project::open(path, &options)?;
        let report = project.signatures(verifier);
        Ok((project, report))
    }

    /// The state of every signature of the file.
    pub fn signatures(&self, verifier: Option<&dyn Verifier>) -> SignatureReport {
        SignatureReport {
            project: self.project_signature(verifier),
            branches: self
                .branches
                .values()
                .filter(|b| !b.is_extension())
                .map(|b| (b.id.clone(), self.status_of_branch(b, verifier)))
                .collect(),
        }
    }

    /// The state of the signature of the branch `id`; `None` when there is no such branch or it
    /// is an extension folder.
    pub fn branch_signature(
        &self,
        id: &str,
        verifier: Option<&dyn Verifier>,
    ) -> Option<SignatureStatus> {
        self.branches
            .get(id)
            .filter(|b| !b.is_extension())
            .map(|b| self.status_of_branch(b, verifier))
    }

    /// The state of the signature of the project record. A signature whose index no longer
    /// states what the branches hold is wrong, whatever its bytes say.
    pub fn project_signature(&self, verifier: Option<&dyn Verifier>) -> SignatureStatus {
        let signature = self.record.signature.as_ref();
        if signature.is_none() {
            return SignatureStatus::Unsigned;
        }
        if !self.verified {
            return SignatureStatus::Unchecked;
        }
        if !self.index_matches() {
            return SignatureStatus::Invalid;
        }
        match try_signing_message(PROJECT_SIGNATURE_LABEL, &self.record) {
            Some(message) => status_of(signature, &message, verifier),
            None => SignatureStatus::Invalid,
        }
    }

    fn status_of_branch(
        &self,
        branch: &Branch,
        verifier: Option<&dyn Verifier>,
    ) -> SignatureStatus {
        let Some(record) = &branch.record else {
            // a record that cannot be read may hide a damaged signature
            return if branch.files.contains_key(BRANCH_FILE) {
                SignatureStatus::Invalid
            } else {
                SignatureStatus::Unsigned
            };
        };
        if record.signature.is_none() {
            return SignatureStatus::Unsigned;
        }
        if !self.verified {
            return SignatureStatus::Unchecked;
        }
        // the signature covers the record, and the record covers the files only through these
        if record.branch != branch.id || record.content_hash != branch.hash {
            return SignatureStatus::Invalid;
        }
        match try_signing_message(BRANCH_SIGNATURE_LABEL, record) {
            Some(message) => status_of(record.signature.as_ref(), &message, verifier),
            None => SignatureStatus::Invalid,
        }
    }

    /// True when the index of `project.json` states every branch with the hash it has now.
    fn index_matches(&self) -> bool {
        let index: BTreeMap<&str, &str> = self
            .record
            .branches
            .iter()
            .map(|b| (b.id.as_str(), b.content_hash.as_str()))
            .collect();
        index.len() == self.record.branches.len()
            && index
                .iter()
                .all(|(id, hash)| self.branches.get(*id).is_some_and(|b| b.hash == *hash))
            && self
                .branches
                .values()
                .filter(|b| !b.is_extension())
                .all(|b| index.contains_key(b.id.as_str()))
    }

    /// The record of a branch that may be signed: present, sound, and not sealed.
    fn signable_record(&self, id: &str) -> Result<BranchRecord> {
        if self.record.seal.is_some() {
            return Err(refuse(PkgCode::Sealed, "the project is sealed read-only"));
        }
        if !self.verified {
            return Err(refuse(
                PkgCode::Integrity,
                "the entries were not checked, so nothing is signed",
            ));
        }
        let branch = self
            .branches
            .get(id)
            .ok_or_else(|| refuse_at(PkgCode::BranchRef, "there is no such branch", id))?;
        let record = branch
            .record
            .as_ref()
            .filter(|r| !branch.is_extension() && r.branch == id)
            .ok_or_else(|| {
                refuse_at(
                    PkgCode::BranchRecord,
                    "the branch has no valid record to sign",
                    id,
                )
            })?;
        if record.seal.is_some() {
            return Err(refuse_at(
                PkgCode::Sealed,
                "the branch is sealed read-only",
                id,
            ));
        }
        if record.content_hash != branch.hash {
            return Err(refuse_at(
                PkgCode::BranchHash,
                "the files of the branch are not the ones its record hashes",
                id,
            ));
        }
        Ok(record.clone())
    }

    /// Says why [`Project::sign_branch`] would refuse, without writing anything.
    pub fn check_signable(&self, id: &str) -> Result<()> {
        self.signable_record(id).map(|_| ())
    }

    /// Signs the branch `id` as it is. Only its `branch.json` changes (and the audit log when a
    /// time is given); the project record is not touched, so a signature of the project keeps
    /// holding. Signing is not writing the branch: any program may do it for the person who
    /// holds the key. A sealed branch or project is refused (020). `options.modified` and
    /// `options.signer` are not used.
    pub fn sign_branch(
        mut self,
        id: &str,
        signer: &dyn Signer,
        options: &SaveOptions<'_>,
    ) -> Result<Project> {
        let mut record = self.signable_record(id)?;
        record.signature = None;
        record.signature = Some(signature_of(signer, BRANCH_SIGNATURE_LABEL, &record, id)?);
        let bytes = to_canonical_file_bytes(&record)
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
        let mut plan = Plan::default();
        let name = entry_name(id, BRANCH_FILE);
        plan.skip.insert(name.clone());
        plan.extra.push((name, bytes));
        if let Some(at) = &options.audit_at {
            let line = serde_json::json!({
                "action": "sign", "at": at, "branch": id, "key": signer.key_id(),
            });
            self.append_audit(&mut plan, &line)?;
        }
        let record = self.record.clone();
        self.rewrite(plan, &record, &options.atomic)
    }

    /// Adds one line to the audit log in what is about to be written.
    fn append_audit(&mut self, plan: &mut Plan, line: &Value) -> Result<()> {
        let mut log = if self.container.entry(AUDIT_LOG).is_some() {
            self.container.read_limited(AUDIT_LOG, MAX_AUDIT)?
        } else {
            Vec::new()
        };
        let text = eh3d_container::canonical_string(line)
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
        log.extend_from_slice(text.as_bytes());
        log.push(b'\n');
        plan.skip.insert(AUDIT_LOG.to_owned());
        plan.extra.push((AUDIT_LOG.to_owned(), log));
        Ok(())
    }

    /// Writes the file again as `plan` and `record` say and opens it.
    fn rewrite(
        self,
        plan: Plan,
        record: &ProjectRecord,
        atomic: &AtomicOptions,
    ) -> Result<Project> {
        let Project {
            path,
            container,
            spec,
            ..
        } = self;
        assemble(Some(container), &path, &spec, plan, record, atomic)?;
        Project::open(
            &path,
            &OpenOptions {
                spec,
                verify_integrity: true,
            },
        )
    }

    /// Saves one branch. Only this branch and the record of the project change; every other
    /// entry, known or not, is copied through and keeps its bytes. Refused: a branch the writer
    /// does not own (017), a branch or project that is sealed (020), a malformed update (015),
    /// a reference to a branch that is not in the file (018). The file is replaced atomically
    /// and the project is opened again from the new file.
    pub fn save_branch(
        self,
        writer: &Writer,
        update: BranchUpdate,
        options: &SaveOptions<'_>,
    ) -> Result<Project> {
        let id = update.id.clone();
        self.check_may_write(writer, &id, &update.format, &update.app_version)?;
        let mut new_files = Vec::new();
        for (rel, bytes) in &update.entries {
            if crate::ed25519::looks_like_secret_key(bytes) {
                return Err(refuse_at(
                    PkgCode::BranchRecord,
                    "a secret key is never put into a project",
                    &entry_name(&id, rel),
                ));
            }
            if rel == BRANCH_FILE || rel.is_empty() {
                return Err(refuse_at(
                    PkgCode::BranchRecord,
                    "branch.json is written by the container, not given",
                    &entry_name(&id, rel),
                ));
            }
            let name = entry_name(&id, rel);
            check_entry_name(&name)
                .map_err(|_| refuse_at(PkgCode::InvalidName, "the name is not portable", &name))?;
            new_files.push((
                rel.clone(),
                eh3d_container::sha256_hex(bytes),
                bytes.len() as u64,
            ));
        }
        for dep in &update.depends {
            let bad = |why: &str| {
                refuse_at(
                    PkgCode::BranchRef,
                    format!("a reference to '{}': {why}", dep.branch),
                    &id,
                )
            };
            if !valid_branch_id(&dep.branch) || dep.branch == id {
                return Err(bad("it does not name another branch"));
            }
            if !is_sha256_hex(&dep.hash)
                || parse_major_minor(&dep.format).is_none()
                || dep.artifact.is_empty()
            {
                return Err(bad("its hash, format or artifact is malformed"));
            }
            if !self.branches.contains_key(&dep.branch) {
                return Err(bad("that branch is not in this file"));
            }
        }

        let hash = branch_content_hash(
            new_files
                .iter()
                .map(|(n, s, z)| (n.as_str(), *z, s.as_str())),
        );
        let record = BranchRecord {
            schema: RECORD_SCHEMA,
            branch: id.clone(),
            app: writer.app.clone(),
            format: update.format.clone(),
            app_version: update.app_version.clone(),
            content_hash: hash,
            depends: update.depends.clone(),
            seal: update.seal.clone(),
            signature: None,
            encryption: update.encryption.clone(),
            extra: update.extra.clone(),
        };

        let mut plan = Plan::default();
        let prefix = branch_dir(&id);
        plan.skip.extend(
            self.container
                .entries()
                .filter(|e| e.name.starts_with(&prefix))
                .map(|e| e.name.clone()),
        );
        for (rel, bytes) in update.entries {
            plan.extra.push((entry_name(&id, &rel), bytes));
        }
        self.commit_branch(record, plan, options)
    }

    /// Why `writer` may not write the branch `id`, if it may not: a sealed project or branch (020), a name that is not a branch
    /// (015), a branch of another program (017), a format or a program version that is not usable (015).
    pub(crate) fn check_may_write(
        &self,
        writer: &Writer,
        id: &str,
        format: &str,
        app_version: &str,
    ) -> Result<()> {
        if self.record.seal.is_some() {
            return Err(refuse(PkgCode::Sealed, "the project is sealed read-only"));
        }
        if !valid_branch_id(id) {
            return Err(refuse_at(
                PkgCode::BranchRecord,
                "not a valid branch name",
                id,
            ));
        }
        if writer.app.trim().is_empty() {
            return Err(refuse(PkgCode::BranchOwner, "the writer has no name"));
        }
        let shared = is_shared_branch(id);
        if !shared && !writer.claims(id) {
            return Err(refuse_at(
                PkgCode::BranchOwner,
                format!("'{}' does not own this branch", writer.app),
                id,
            ));
        }
        if let Some(record) = self.branches.get(id).and_then(|old| old.record.as_ref()) {
            if !shared && record.app != writer.app {
                return Err(refuse_at(
                    PkgCode::BranchOwner,
                    format!("the branch was written by '{}'", record.app),
                    id,
                ));
            }
            if record.seal.is_some() {
                return Err(refuse_at(
                    PkgCode::Sealed,
                    "the branch is sealed read-only",
                    id,
                ));
            }
        }
        if parse_major_minor(format).is_none() || app_version.trim().is_empty() {
            return Err(refuse_at(
                PkgCode::BranchRecord,
                "the format must be major.minor and the program version is needed",
                id,
            ));
        }
        Ok(())
    }

    /// Saves a branch from files that are on disk, with the same rules as [`Project::save_branch`]. The files that are kept keep
    /// their bytes.
    pub(crate) fn save_streamed(
        self,
        writer: &Writer,
        update: StreamedUpdate,
        options: &SaveOptions<'_>,
    ) -> Result<Project> {
        let id = update.id.clone();
        self.check_may_write(writer, &id, &update.format, &update.app_version)?;
        let mut files: Vec<(String, u64, String)> = Vec::new();
        if let Some(old) = self.branches.get(&id) {
            for rel in &update.keep {
                let info = old.files.get(rel).ok_or_else(|| {
                    refuse_at(
                        PkgCode::OutsideRoots,
                        "there is no such file in the branch",
                        &entry_name(&id, rel),
                    )
                })?;
                files.push((rel.clone(), info.size, info.sha256.clone()));
            }
        }
        for (rel, bytes) in &update.entries {
            if crate::ed25519::looks_like_secret_key(bytes) {
                return Err(refuse_at(
                    PkgCode::BranchRecord,
                    "a secret key is never put into a project",
                    &entry_name(&id, rel),
                ));
            }
            files.push((
                rel.clone(),
                bytes.len() as u64,
                eh3d_container::sha256_hex(bytes),
            ));
        }
        for file in &update.files {
            files.push((file.rel.clone(), file.size, file.sha256.clone()));
        }
        for (rel, _, _) in &files {
            let name = entry_name(&id, rel);
            if rel == BRANCH_FILE || rel.is_empty() || check_entry_name(&name).is_err() {
                return Err(refuse_at(
                    PkgCode::InvalidName,
                    "the name is not portable",
                    &name,
                ));
            }
        }
        let hash = branch_content_hash(files.iter().map(|(n, z, s)| (n.as_str(), *z, s.as_str())));
        let old = self.branches.get(&id).and_then(|b| b.record.as_ref());
        let record = BranchRecord {
            schema: RECORD_SCHEMA,
            branch: id.clone(),
            app: writer.app.clone(),
            format: update.format.clone(),
            app_version: update.app_version.clone(),
            content_hash: hash,
            depends: old.map(|r| r.depends.clone()).unwrap_or_default(),
            seal: None,
            signature: None,
            encryption: None,
            extra: update.extra,
        };
        let kept: BTreeSet<String> = update.keep.iter().map(|rel| entry_name(&id, rel)).collect();
        let mut plan = Plan::default();
        let prefix = branch_dir(&id);
        plan.skip.extend(
            self.container
                .entries()
                .filter(|e| e.name.starts_with(&prefix) && !kept.contains(&e.name))
                .map(|e| e.name.clone()),
        );
        for (rel, bytes) in update.entries {
            plan.extra.push((entry_name(&id, &rel), bytes));
        }
        for mut file in update.files {
            file.rel = entry_name(&id, &file.rel);
            plan.streams.push(file);
        }
        self.commit_branch(record, plan, options)
    }

    /// Signs and writes the record of one branch, the audit line and the index of `project.json`, and replaces the file.
    fn commit_branch(
        mut self,
        mut record: BranchRecord,
        mut plan: Plan,
        options: &SaveOptions<'_>,
    ) -> Result<Project> {
        let id = record.branch.clone();
        if let Some(signer) = options.signer {
            record.signature = Some(signature_of(signer, BRANCH_SIGNATURE_LABEL, &record, &id)?);
        }
        let record_bytes = to_canonical_file_bytes(&record)
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
        plan.extra
            .push((entry_name(&id, BRANCH_FILE), record_bytes));
        if let Some(at) = &options.audit_at {
            let line = serde_json::json!({
                "action": "save", "app": record.app, "app_version": record.app_version, "at": at, "branch": id,
            });
            self.append_audit(&mut plan, &line)?;
        }

        // The index of project.json: the other branches keep the line they had.
        let mut index: BTreeMap<String, BranchIndexEntry> = BTreeMap::new();
        let old_index: BTreeMap<&str, &BranchIndexEntry> = self
            .record
            .branches
            .iter()
            .map(|b| (b.id.as_str(), b))
            .collect();
        for branch in self
            .branches
            .values()
            .filter(|b| !b.is_extension() && b.id != id)
        {
            let entry = match (old_index.get(branch.id.as_str()), &branch.record) {
                (Some(old), _) => (*old).clone(),
                (None, Some(r)) => index_line(&branch.id, r),
                (None, None) => BranchIndexEntry {
                    id: branch.id.clone(),
                    app: String::new(),
                    format: String::new(),
                    app_version: String::new(),
                    content_hash: branch.hash.clone(),
                    extra: BTreeMap::new(),
                },
            };
            index.insert(branch.id.clone(), entry);
        }
        let mut line = index_line(&id, &record);
        if let Some(old) = old_index.get(id.as_str()) {
            line.extra = old.extra.clone();
        }
        index.insert(id.clone(), line);
        let mut project_record = self.record.clone();
        project_record.branches = index.into_values().collect();
        if options.modified.is_some() {
            project_record.modified = options.modified.clone();
        }
        project_record.signature = None;

        self.rewrite(plan, &project_record, &options.atomic)
    }

    /// Seals the whole project read-only. Any program may do it; nothing can be saved after.
    pub fn seal_project(self, seal: Seal, options: &SaveOptions<'_>) -> Result<Project> {
        if self.record.seal.is_some() {
            return Err(refuse(PkgCode::Sealed, "the project is already sealed"));
        }
        let mut record = self.record.clone();
        record.seal = Some(seal);
        record.signature = None;
        if options.modified.is_some() {
            record.modified = options.modified.clone();
        }
        self.rewrite(Plan::default(), &record, &options.atomic)
    }

    /// Signs the project record (which holds the hash of every branch) with `signer`.
    pub fn sign_project(self, signer: &dyn Signer, options: &SaveOptions<'_>) -> Result<Project> {
        if self.record.seal.is_some() {
            return Err(refuse(PkgCode::Sealed, "the project is sealed read-only"));
        }
        if !self.verified || !self.index_matches() {
            return Err(refuse_at(
                PkgCode::BranchHash,
                "the index does not state what the branches hold, so nothing is signed",
                "project.json",
            ));
        }
        let mut record = self.record.clone();
        record.signature = None;
        record.signature = Some(signature_of(
            signer,
            PROJECT_SIGNATURE_LABEL,
            &record,
            "project.json",
        )?);
        self.rewrite(Plan::default(), &record, &options.atomic)
    }

    /// Writes the `manufacturing` branch out as a `.eh3dproj` 1.0.0 package, byte for byte the
    /// package that [`manufacturing_update`] took it from.
    pub fn export_manufacturing(&mut self, out: &Path, atomic: &AtomicOptions) -> Result<()> {
        let branch = self.branches.get(MANUFACTURING_BRANCH).ok_or_else(|| {
            refuse_at(
                PkgCode::BranchRef,
                "the project has no manufacturing branch",
                MANUFACTURING_BRANCH,
            )
        })?;
        let rels: Vec<String> = branch
            .files
            .keys()
            .filter(|r| *r != BRANCH_FILE)
            .cloned()
            .collect();
        let dir = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut writer = ContainerWriter::new(&manufacturing(), dir);
        for rel in &rels {
            let name = entry_name(MANUFACTURING_BRANCH, rel);
            let mut stream = self.container.read_stream(&name)?;
            writer.add_stream(rel, &mut stream)?;
        }
        writer.write_to_path(out, atomic)
    }

    /// Takes the files of a branch out into `directory`, `branch.json` included, byte for byte
    /// (an encrypted branch comes out as it is stored). Files that already exist are not
    /// replaced. Returns the number of files written.
    pub fn extract_branch(&mut self, id: &str, directory: &Path) -> Result<usize> {
        let branch = self
            .branches
            .get(id)
            .ok_or_else(|| refuse_at(PkgCode::BranchRef, "there is no such branch", id))?;
        let rels: Vec<String> = branch.files.keys().cloned().collect();
        let mut count = 0;
        for rel in rels {
            let target = directory.join(&rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            let mut stream = self.container.read_stream(&entry_name(id, &rel))?;
            io::copy(&mut stream, &mut file)?;
            count += 1;
        }
        Ok(count)
    }
}

/// Reads a `.eh3dproj` package as the content of the `manufacturing` branch (format `1.0`).
/// The package is read under the rules of its own format and must pass them; its names must
/// also be acceptable under the strict rules of the project container.
pub fn manufacturing_update(package: &Path, app_version: &str) -> Result<BranchUpdate> {
    let spec = manufacturing();
    let mut container = Container::open_path(package, &spec)?;
    container.check_layout()?;
    container.verify_integrity()?;
    let names: Vec<String> = container.entries().map(|e| e.name.clone()).collect();
    let mut update = BranchUpdate::new(MANUFACTURING_BRANCH, "1.0", app_version);
    for name in names {
        if name == "integrity.json" {
            continue;
        }
        let limit = spec.limits.max_entry_bytes_for(&name);
        let bytes = container.read_limited(&name, limit)?;
        update.entries.insert(name, bytes);
    }
    Ok(update)
}

fn index_line(id: &str, record: &BranchRecord) -> BranchIndexEntry {
    BranchIndexEntry {
        id: id.to_owned(),
        app: record.app.clone(),
        format: record.format.clone(),
        app_version: record.app_version.clone(),
        content_hash: record.content_hash.clone(),
        extra: BTreeMap::new(),
    }
}

/// The signature record of `record`, refused when it has no canonical form.
fn signature_of(
    signer: &dyn Signer,
    label: &str,
    record: &impl serde::Serialize,
    entry: &str,
) -> Result<SignatureRecord> {
    let message = try_signing_message(label, record).ok_or_else(|| {
        refuse_at(
            PkgCode::BranchRecord,
            "the record has a number with no canonical form and cannot be signed",
            entry,
        )
    })?;
    Ok(sign_with(signer, &message))
}

fn judge_signature(
    status: &SignatureStatus,
    required: bool,
    branch: Option<&str>,
    entry: &str,
    add: &mut impl FnMut(PkgCode, String, Option<&str>, Option<String>),
) {
    let entry = Some(entry.to_owned());
    match status {
        SignatureStatus::Valid(_) => {}
        SignatureStatus::Invalid => add(
            PkgCode::SignatureInvalid,
            "the signature does not verify".into(),
            branch,
            entry,
        ),
        SignatureStatus::Unsigned if required => add(
            PkgCode::SignatureMissing,
            "a signature is required and there is none".into(),
            branch,
            entry,
        ),
        SignatureStatus::UnknownKey if required => add(
            PkgCode::SignatureMissing,
            "the key of the signature is not known".into(),
            branch,
            entry,
        ),
        SignatureStatus::Unchecked if required => add(
            PkgCode::SignatureMissing,
            "a signature is required and it could not be checked".into(),
            branch,
            entry,
        ),
        _ => {}
    }
}

/// What to leave out of the copy and what to add (`streams` name the entry in full and come from files on disk).
#[derive(Default)]
struct Plan {
    skip: BTreeSet<String>,
    extra: Vec<(String, Vec<u8>)>,
    streams: Vec<StreamedFile>,
}

/// Writes the new file: every entry of the old container that is not skipped, copied through
/// as a stream (so its bytes are written again exactly as the deterministic writer writes
/// them), the new entries and the project record. The old file is closed before the target is
/// replaced, which Windows needs.
fn assemble(
    container: Option<Container<File>>,
    path: &Path,
    spec: &ContainerSpec,
    plan: Plan,
    record: &ProjectRecord,
    atomic: &AtomicOptions,
) -> Result<()> {
    let mut container = container;
    let copy: Vec<String> = container
        .iter()
        .flat_map(|c| c.entries().map(|e| e.name.clone()))
        .filter(|n| n != "project.json" && n != "integrity.json" && !plan.skip.contains(n))
        .collect();
    let mut roots: BTreeSet<String> = BTreeSet::new();
    for name in copy
        .iter()
        .chain(plan.extra.iter().map(|(n, _)| n))
        .chain(plan.streams.iter().map(|s| &s.rel))
    {
        if let Some(root) = root_of(name) {
            roots.insert(root);
        }
    }
    let mut writer_spec = spec.clone();
    writer_spec.allowed_roots = roots.into_iter().collect();
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut writer = ContainerWriter::new(&writer_spec, dir);
    if let Some(open) = container.as_mut() {
        for name in &copy {
            let mut stream = open.read_stream(name)?;
            writer.add_stream(name, &mut stream)?;
        }
    }
    for (name, bytes) in &plan.extra {
        writer.add(name, bytes)?;
    }
    for stream in &plan.streams {
        let mut file = File::open(&stream.path)?;
        file.seek(SeekFrom::Start(stream.offset))?;
        writer.add_stream(&stream.rel, file.take(stream.size))?;
    }
    if !plan.streams.is_empty() {
        let written = writer.integrity();
        for stream in &plan.streams {
            let same = written
                .files
                .get(&stream.rel)
                .is_some_and(|e| e.size == stream.size && e.sha256 == stream.sha256);
            if !same {
                return Err(ContainerError::Io(io::Error::other(
                    "a file changed while it was being added; nothing was written",
                )));
            }
        }
    }
    let project = to_canonical_file_bytes(record)
        .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
    writer.add("project.json", &project)?;
    drop(container);
    writer.write_to_path(path, atomic)
}

/// Reads all of a stream into memory for callers that hold a small entry (a helper for tests
/// and tools; the limits of the container still apply to the stream).
pub fn read_all(mut stream: impl Read) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    stream.read_to_end(&mut out)?;
    Ok(out)
}
