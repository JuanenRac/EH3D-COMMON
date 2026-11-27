//! What a program does with its own branch of a `.eh3d` project, written once.
//!
//! A program says which branch is its own, the format of that branch, the name of the record that
//! says which file is what and its version ([`BranchKit`]). With that it can:
//!
//! * **save**: [`BranchKit::update`] opens or creates the project, hands over the branch as it is
//!   (a [`WorkingCopy`]) and writes it back; what is not touched stays;
//! * **open**: [`BranchKit::open`] reads its branch with what is out of date or wrong;
//! * **import** a file of another program: [`BranchKit::import`] keeps the original under
//!   `origin/` with the report of what was read, what was not and what an export loses;
//! * **take** from another branch: [`BranchKit::take`] keeps a copy under `taken/` with a
//!   reference that remembers the content hash and the format of its source, so a later change
//!   of the source is told as **out of date** ([`warnings`], [`list`]).
//!
//! A program checks what its own formats mean (a drawing, a program of a machine) in the closures
//! it passes; this module never interprets a file. Every file is read before anything is written,
//! only the branch of the program changes and nothing in a project is run.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

use eh3d_container::{check_entry_name, sha256_hex, AtomicOptions, ContainerError};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::meta::{ConversionReport, Converter, Counted, SourceFile, ORIGIN_DIR, REPORT_FILE};
use crate::project::{
    BranchUpdate, Checks, NewProject, OpenOptions, Project, SaveOptions, StaleRef, Writer,
};
use crate::records::Dependency;
use crate::revision::Revision;
use crate::spec::BRANCH_FILE;

/// The folder of a branch that holds what was taken from other branches.
pub const TAKEN_DIR: &str = "taken/";
/// The extension of a project.
pub const EXTENSION: &str = "eh3d";
/// The unit of the lengths of a project that a program creates.
pub const UNITS: &str = "millimetres";

/// Why something with a branch could not be done. The message says what to a person.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BranchError {
    /// The container refused or the file system failed.
    #[error("{0}")]
    Container(#[from] ContainerError),
    /// Something about a file the person named.
    #[error("{file}: {why}")]
    File {
        /// The file.
        file: String,
        /// What is wrong with it.
        why: String,
    },
    /// Anything else that is refused, in words.
    #[error("{0}")]
    Refused(String),
}

/// The result of this module.
pub type BranchResult<T> = std::result::Result<T, BranchError>;

fn refused(why: impl Into<String>) -> BranchError {
    BranchError::Refused(why.into())
}

fn in_file(path: &Path, why: impl ToString) -> BranchError {
    BranchError::File {
        file: path.display().to_string(),
        why: why.to_string(),
    }
}

/// Something a person should be told after a branch was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notice {
    /// The project was signed; the change removed the signature.
    ProjectSignatureRemoved,
    /// The branch was signed and was written again without a signer.
    BranchSignatureRemoved,
    /// Another import was there and its original is gone.
    ImportReplaced,
}

impl std::fmt::Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Notice::ProjectSignatureRemoved => {
                "the signature of the project does not hold after a change and is removed"
            }
            Notice::BranchSignatureRemoved => {
                "the signature of the branch does not hold after a change and is removed"
            }
            Notice::ImportReplaced => "the import that was there before was replaced",
        })
    }
}

/// The name and the identifier of a project that is being created.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The name (the name of the file when there is none).
    pub name: Option<String>,
    /// The identifier (a random UUID when there is none).
    pub id: Option<String>,
}

/// The file name of a path, as text.
pub fn file_name(path: &Path) -> BranchResult<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| in_file(path, "the file has no usable name"))
}

/// The extension of a name in lower case, or nothing.
pub fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Reads a file whole, never more than `limit` bytes of it.
pub fn read_bounded(path: &Path, limit: u64) -> BranchResult<Vec<u8>> {
    let file = fs::File::open(path).map_err(|e| in_file(path, e))?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| in_file(path, e))?;
    if bytes.len() as u64 > limit {
        return Err(in_file(
            path,
            format!("the file is larger than {limit} bytes"),
        ));
    }
    Ok(bytes)
}

/// A random identifier in the shape of a UUID of version 4.
pub fn new_project_id() -> BranchResult<String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|_| refused("the system gave no random bytes"))?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex = crate::records::to_hex(&b);
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

/// Opens the project at `path`, every entry checked.
pub fn open_project(path: &Path) -> BranchResult<Project> {
    Project::open(path, &OpenOptions::default()).map_err(|e| in_file(path, e))
}

/// Opens the project at `path` to write to it, refused (`EH3D-PKG-025`) when it no longer holds
/// what was read as `seen`: another program saved it meanwhile. Writes of this program do not
/// count; see [`crate::revision`].
pub fn open_unchanged(path: &Path, seen: &Revision) -> BranchResult<Project> {
    crate::revision::check_unchanged(path, seen)?;
    let project = open_project(path)?;
    // what was opened is judged too: the file may have been saved between the look and the opening
    if !crate::revision::follows(path, seen, &project.revision()) {
        return Err(crate::revision::changed().into());
    }
    Ok(project)
}

/// Opens the project at `path`, or creates it when there is no file there. A name or an
/// identifier are only for a project that is being created.
pub fn open_or_create(path: &Path, identity: &Identity) -> BranchResult<Project> {
    if path.exists() {
        if identity.name.is_some() || identity.id.is_some() {
            return Err(refused(
                "--name and --id are only for a project that is being created",
            ));
        }
        return open_project(path);
    }
    if extension_of(&file_name(path)?) != EXTENSION {
        return Err(in_file(
            path,
            format!("a new project is a file that ends in .{EXTENSION}"),
        ));
    }
    let stem = path.file_stem().map_or_else(
        || "project".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let id = match &identity.id {
        Some(id) => id.clone(),
        None => new_project_id()?,
    };
    let new = NewProject {
        id,
        name: identity.name.clone().unwrap_or(stem),
        units: UNITS.to_string(),
        created: None,
    };
    Project::create(
        path,
        new,
        &OpenOptions::default(),
        &AtomicOptions::default(),
    )
    .map_err(|e| in_file(path, e))
}

/// One count of a conversion report.
pub fn counted(item: &str, count: usize, reason: Option<&str>) -> Counted {
    Counted {
        item: item.to_string(),
        count: count as u64,
        reason: reason.map(str::to_owned),
    }
}

/// What a conversion read of a file of another program, for [`conversion_report`].
#[derive(Debug, Default, Clone)]
pub struct Conversion {
    /// What was read.
    pub read: Vec<Counted>,
    /// What was not read, with the reason.
    pub not_read: Vec<Counted>,
    /// What writing the file again cannot say.
    pub export_losses: Vec<Counted>,
}

/// The report of the import of the file `name` with these bytes, by the program `converter` of
/// version `version`. The original is the one [`BranchKit::import`] keeps.
pub fn conversion_report(
    converter: &str,
    version: &str,
    format: &str,
    name: &str,
    bytes: &[u8],
    conversion: Conversion,
) -> ConversionReport {
    ConversionReport {
        schema: 1,
        source: SourceFile {
            format: format.to_string(),
            name: name.to_string(),
            sha256: sha256_hex(bytes),
            size: bytes.len() as u64,
        },
        converter: Converter {
            name: converter.to_string(),
            version: version.to_string(),
        },
        original: Some(format!("{ORIGIN_DIR}{name}")),
        read: conversion.read,
        not_read: conversion.not_read,
        export_losses: conversion.export_losses,
        extra: BTreeMap::new(),
    }
}

/// The branch of a program as it is read, changed and written back. Names are relative to the
/// branch. The record of the branch ([`BranchKit::manifest`]) is a JSON object whose keys the
/// program chooses.
#[derive(Debug, Clone, Default)]
pub struct WorkingCopy {
    branch: String,
    files: BTreeMap<String, Vec<u8>>,
    manifest: Map<String, Value>,
    depends: Vec<Dependency>,
    extra: BTreeMap<String, Value>,
    signed: bool,
}

impl WorkingCopy {
    /// The files of the branch, `branch.json` left out.
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }

    /// One file.
    pub fn file(&self, rel: &str) -> Option<&[u8]> {
        self.files.get(rel).map(Vec::as_slice)
    }

    /// The references to other branches.
    pub fn depends(&self) -> &[Dependency] {
        &self.depends
    }

    /// A text of the record of the branch.
    pub fn text(&self, key: &str) -> Option<String> {
        self.manifest
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// Writes a text in the record of the branch.
    pub fn set(&mut self, key: &str, value: &str) {
        self.manifest
            .insert(key.to_owned(), Value::from(value.to_owned()));
    }

    /// A list of texts of the record of the branch (empty when there is none).
    pub fn list(&self, key: &str) -> Vec<String> {
        self.manifest
            .get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Adds a text to a list of the record, once.
    pub fn add_to_list(&mut self, key: &str, value: &str) {
        let mut list = self.list(key);
        if !list.iter().any(|v| v == value) {
            list.push(value.to_owned());
        }
        self.manifest.insert(key.to_owned(), Value::from(list));
    }

    /// Puts a file in the branch, checking that its name can live in a project.
    pub fn put(&mut self, rel: &str, bytes: Vec<u8>) -> BranchResult<()> {
        check_rel(&self.branch, rel)?;
        self.files.insert(rel.to_owned(), bytes);
        Ok(())
    }

    /// Takes a file out of the branch, with the references that only it used.
    pub fn remove(&mut self, rel: &str) {
        self.files.remove(rel);
        // a reference that never named files stays; one that lost its last file goes
        self.depends.retain_mut(|dep| {
            let used = !dep.uses.is_empty();
            dep.uses.retain(|u| u != rel);
            !(used && dep.uses.is_empty())
        });
    }

    /// Adds a reference to another branch.
    pub fn add_dependency(&mut self, dependency: Dependency) {
        self.depends.push(dependency);
    }

    /// Makes `rel` the file that the record names under `key`. The one before goes when it was a
    /// copy of the program's own (under `own_dir`) or a taken one; the original of an import
    /// stays with its report.
    pub fn choose(&mut self, key: &str, own_dir: &str, rel: &str) {
        if let Some(old) = self.text(key) {
            if old != rel && (old.starts_with(own_dir) || old.starts_with(TAKEN_DIR)) {
                self.remove(&old);
            }
        }
        self.set(key, rel);
    }

    /// Forgets each of these names of the record that points at a file that is not there.
    pub fn forget_missing(&mut self, keys: &[&str]) {
        for key in keys {
            if self.text(key).is_some_and(|v| !self.files.contains_key(&v)) {
                self.manifest.remove(*key);
            }
        }
    }

    /// The report of the last import, when there is one.
    pub fn report(&self) -> BranchResult<Option<ConversionReport>> {
        self.files
            .get(REPORT_FILE)
            .map(|bytes| {
                serde_json::from_slice(bytes).map_err(|e| {
                    refused(format!(
                        "{}/{REPORT_FILE} is not a valid record: {e}",
                        self.branch
                    ))
                })
            })
            .transpose()
    }
}

fn check_rel(branch: &str, rel: &str) -> BranchResult<()> {
    if rel == BRANCH_FILE || check_entry_name(&format!("{branch}/{rel}")).is_err() {
        return Err(refused(format!(
            "'{rel}' is not a name that can be kept in a project"
        )));
    }
    Ok(())
}

/// What a program says once about its branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BranchKit<'a> {
    /// The branch the program owns.
    pub branch: &'a str,
    /// The format of the branch, `major.minor`; a branch of another format is refused (024).
    pub format: &'a str,
    /// The file of the branch that says which of its files is what.
    pub manifest: &'a str,
    /// The version of the program.
    pub app_version: &'a str,
}

impl<'a> BranchKit<'a> {
    /// The kit of a program.
    pub const fn new(
        branch: &'a str,
        format: &'a str,
        manifest: &'a str,
        app_version: &'a str,
    ) -> BranchKit<'a> {
        BranchKit {
            branch,
            format,
            manifest,
            app_version,
        }
    }

    /// An empty working copy of the branch.
    pub fn empty(&self) -> WorkingCopy {
        WorkingCopy {
            branch: self.branch.to_owned(),
            ..WorkingCopy::default()
        }
    }

    /// Says whether a file of this name can be put in the branch, before anything is written.
    pub fn check_name(&self, rel: &str) -> BranchResult<()> {
        check_rel(self.branch, rel)
    }

    /// Reads the branch of the program, empty when the project has none yet.
    pub fn load(&self, project: &mut Project) -> BranchResult<WorkingCopy> {
        let Some(branch) = project.branch(self.branch) else {
            return Ok(self.empty());
        };
        let record = branch.record.clone().ok_or_else(|| {
            refused(format!(
                "EH3D-PKG-015: {}/{BRANCH_FILE} is missing or not valid",
                self.branch
            ))
        })?;
        if record.format != self.format {
            return Err(refused(format!(
                "EH3D-PKG-024: the branch {} has the format {}, and this program handles {}",
                self.branch, record.format, self.format
            )));
        }
        let names: Vec<String> = branch
            .files
            .keys()
            .filter(|n| *n != BRANCH_FILE)
            .cloned()
            .collect();
        let mut copy = WorkingCopy {
            depends: record.depends,
            extra: record.extra,
            signed: record.signature.is_some(),
            ..self.empty()
        };
        for name in names {
            let bytes = project.read_entry(self.branch, &name)?;
            copy.files.insert(name, bytes);
        }
        if let Some(bytes) = copy.files.get(self.manifest) {
            copy.manifest = serde_json::from_slice(bytes).map_err(|e| {
                refused(format!(
                    "{}/{} is not a valid record: {e}",
                    self.branch, self.manifest
                ))
            })?;
        }
        Ok(copy)
    }

    /// Writes the branch. Nothing else of the project changes but its two records (and the audit
    /// log when `options` gives a time). References to branches that are gone are dropped.
    pub fn write(
        &self,
        project: Project,
        mut copy: WorkingCopy,
        options: &SaveOptions<'_>,
    ) -> BranchResult<(Project, Vec<Notice>)> {
        let mut notices = Vec::new();
        if project.record().signature.is_some() {
            notices.push(Notice::ProjectSignatureRemoved);
        }
        if copy.signed && options.signer.is_none() {
            notices.push(Notice::BranchSignatureRemoved);
        }
        copy.manifest.insert("schema".to_string(), Value::from(1));
        let text = pretty_line(&copy.manifest)?;
        copy.files.insert(self.manifest.to_string(), text);
        let mut update = BranchUpdate::new(self.branch, self.format, self.app_version);
        update.depends = copy
            .depends
            .into_iter()
            .filter(|d| project.branch(&d.branch).is_some())
            .collect();
        update.entries = copy.files;
        update.extra = copy.extra;
        let saved = project.save_branch(&Writer::for_app(self.branch), update, options)?;
        Ok((saved, notices))
    }

    /// Opens or creates the project, lets `apply` change the branch and writes it. Read and check
    /// every file before calling this, so a refusal leaves no half-made project.
    pub fn update(
        &self,
        path: &Path,
        identity: &Identity,
        apply: impl FnOnce(&mut Project, &mut WorkingCopy) -> BranchResult<()>,
    ) -> BranchResult<(Project, Vec<Notice>)> {
        let mut project = open_or_create(path, identity)?;
        let mut copy = self.load(&mut project)?;
        apply(&mut project, &mut copy)?;
        self.write(project, copy, &SaveOptions::default())
    }

    /// Like [`BranchKit::update`] for a project that is there and was read before, as `seen`
    /// ([`Project::revision`]): refused (`EH3D-PKG-025`), with nothing written, when another
    /// program saved the file since. A program offers then to read it again, or to save under
    /// another name.
    pub fn update_since(
        &self,
        path: &Path,
        seen: &Revision,
        apply: impl FnOnce(&mut Project, &mut WorkingCopy) -> BranchResult<()>,
    ) -> BranchResult<(Project, Vec<Notice>)> {
        let mut project = open_unchanged(path, seen)?;
        let mut copy = self.load(&mut project)?;
        apply(&mut project, &mut copy)?;
        self.write(project, copy, &SaveOptions::default())
    }

    /// Imports a file of another program: the original is kept byte for byte under `origin/` with
    /// `report` (see [`conversion_report`]), replacing the import that was there. `apply` gets the
    /// name of the original inside the branch and says what the program does with it.
    pub fn import(
        &self,
        path: &Path,
        identity: &Identity,
        bytes: Vec<u8>,
        report: &ConversionReport,
        apply: impl FnOnce(&mut WorkingCopy, &str) -> BranchResult<()>,
    ) -> BranchResult<Vec<Notice>> {
        let mut original = format!("{ORIGIN_DIR}{}", report.source.name);
        let mut report = report.clone();
        // a file with the name of the report would be written over by it: it is kept under another one
        if original.eq_ignore_ascii_case(REPORT_FILE) {
            original = format!("{ORIGIN_DIR}original-{}", report.source.name);
            report.original = report.original.map(|_| original.clone());
        }
        self.check_name(&original)?;
        let text = pretty_line(&report)?;
        let mut replaced = false;
        let (_, mut notices) = self.update(path, identity, |_, copy| {
            let before: Vec<String> = copy
                .files
                .keys()
                .filter(|n| n.starts_with(ORIGIN_DIR))
                .cloned()
                .collect();
            for old in &before {
                copy.remove(old);
            }
            replaced = before.iter().any(|n| n != REPORT_FILE && *n != original);
            copy.put(&original, bytes)?;
            copy.put(REPORT_FILE, text)?;
            apply(copy, &original)
        })?;
        if replaced {
            notices.push(Notice::ImportReplaced);
        }
        Ok(notices)
    }

    /// Copies the file `rel` of the branch `source` into `taken/<source>/<rel>`, with a reference
    /// that remembers the state of its source. `artifact` says what kind of thing it is, `check`
    /// reads its bytes before anything is written and `apply` gets the new name. Returns that
    /// name.
    pub fn take(
        &self,
        path: &Path,
        source: &str,
        rel: &str,
        artifact: &str,
        check: impl FnOnce(&[u8]) -> BranchResult<()>,
        apply: impl FnOnce(&mut WorkingCopy, &str),
    ) -> BranchResult<(String, Vec<Notice>)> {
        if source == self.branch {
            return Err(refused(
                "a branch takes from another branch, not from itself",
            ));
        }
        let mut project = open_project(path)?;
        let bytes = project.read_entry(source, rel)?;
        check(&bytes)?;
        let target = format!("{TAKEN_DIR}{source}/{rel}");
        self.check_name(&target)?;
        let dependency = project.dependency_on(source, artifact, 1, vec![target.clone()])?;
        let mut copy = self.load(&mut project)?;
        copy.remove(&target);
        copy.put(&target, bytes)?;
        copy.add_dependency(dependency);
        apply(&mut copy, &target);
        let (_, notices) = self.write(project, copy, &SaveOptions::default())?;
        Ok((target, notices))
    }

    /// Reads the branch of the program, with what is out of date or wrong in the project. A
    /// project without the branch is refused.
    pub fn open(&self, path: &Path) -> BranchResult<(Project, WorkingCopy, Vec<String>)> {
        let mut project = open_project(path)?;
        if project.branch(self.branch).is_none() {
            return Err(in_file(
                path,
                format!("the project has no branch {}", self.branch),
            ));
        }
        let copy = self.load(&mut project)?;
        let warnings = warnings(&mut project);
        Ok((project, copy, warnings))
    }
}

/// Pretty JSON with a final line feed, as the records of a branch are written.
fn pretty_line(value: &impl serde::Serialize) -> BranchResult<Vec<u8>> {
    let mut text = serde_json::to_vec_pretty(value)
        .map_err(|e| BranchError::Container(ContainerError::Io(io::Error::other(e))))?;
    text.push(b'\n');
    Ok(text)
}

/// The sentence of an out-of-date derivative.
pub fn stale_text(stale: &StaleRef) -> String {
    format!(
        "OUT OF DATE: '{}' was derived from '{}' ({}: {}) before it changed",
        stale.branch,
        stale.dependency.branch,
        stale.dependency.artifact,
        stale.dependency.uses.join(", ")
    )
}

/// The derived branches whose sources changed and the errors the checks of the container find.
pub fn warnings(project: &mut Project) -> Vec<String> {
    let mut out: Vec<String> = project.stale_refs().iter().map(stale_text).collect();
    for f in project.diagnose(&Checks::default()) {
        if f.is_error() {
            out.push(format!(
                "{}: {} [{}]",
                f.code,
                f.detail,
                f.branch.as_deref().unwrap_or("project")
            ));
        }
    }
    out
}

/// One branch, as a list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The name of the branch.
    pub id: String,
    /// The program that wrote it (`-` without a record).
    pub app: String,
    /// The format of the branch.
    pub format: String,
    /// The version of that program.
    pub app_version: String,
    /// The files of the branch, `branch.json` included.
    pub files: usize,
    /// The content hash.
    pub hash: String,
    /// What is worth saying about it.
    pub notes: Vec<&'static str>,
}

/// What [`list`] finds.
#[derive(Debug, Clone)]
pub struct Listing {
    /// The name of the project.
    pub name: String,
    /// Its identifier.
    pub id: String,
    /// The branches.
    pub rows: Vec<Row>,
    /// The derived branches whose sources changed, and what the checks found wrong.
    pub warnings: Vec<String>,
}

/// The branches of a project, with what is out of date.
pub fn list(path: &Path) -> BranchResult<Listing> {
    let mut project = open_project(path)?;
    let warnings = warnings(&mut project);
    let stale = project.stale_refs();
    let rows = project
        .branches()
        .map(|b| {
            let r = b.record.as_ref();
            let mut notes = Vec::new();
            if !b.is_known() {
                notes.push("not known by name, kept as it is");
            }
            if b.is_sealed() {
                notes.push("sealed");
            }
            if b.is_encrypted() {
                notes.push("encrypted");
            }
            if r.is_some_and(|r| r.signature.is_some()) {
                notes.push("signed");
            }
            if stale.iter().any(|s| s.branch == b.id) {
                notes.push("OUT OF DATE");
            }
            let of = |v: Option<&String>| v.cloned().unwrap_or_else(|| "-".to_string());
            Row {
                id: b.id.clone(),
                app: of(r.map(|r| &r.app)),
                format: of(r.map(|r| &r.format)),
                app_version: of(r.map(|r| &r.app_version)),
                files: b.files.len(),
                hash: b.hash.clone(),
                notes,
            }
        })
        .collect();
    Ok(Listing {
        name: project.record().name.clone(),
        id: project.record().id.clone(),
        rows,
        warnings,
    })
}

/// Writes the files of any branch into a folder, byte for byte (a file that is there is never
/// written over). Returns how many were written.
pub fn extract(path: &Path, branch: &str, dir: &Path) -> BranchResult<usize> {
    let mut project = open_project(path)?;
    Ok(project.extract_branch(branch, dir)?)
}
