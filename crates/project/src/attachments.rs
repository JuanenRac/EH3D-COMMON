//! The attachments of a project: data sheets, manuals, other projects and any other file, kept in the shared branch `attachments/`.
//!
//! * **Any file** may be attached; [`crate::attachfile`] says what kind it is and whether it is dangerous. A dangerous file is
//!   kept like any other, marked, and taken out only when a person has agreed ([`Consent::Given`]). Nothing is ever run, opened or
//!   downloaded here.
//! * A file is stored **once**, under the SHA-256 of its content, which is also its identifier. Many components can point at it
//!   ([`ComponentLink`]: a designator such as `R5`, a value, or a part number with or without its manufacturer), so a data sheet
//!   that a hundred parts use is there one time ([`AttachmentIndex::for_component`]).
//! * It is **inside** the project by default, or only a **link**: an address with the hash of the file it must give.
//! * Files are read and written in blocks and large ones are stored in parts, so an attachment of a gigabyte is never held whole
//!   in memory, and a reader of an older version, which knows nothing of attachments, keeps them byte for byte.
//!
//! The record is `attachments/index.json` and the files are `attachments/blobs/<sha256>/<part>`: no name given by a person is ever a
//! path inside the project. See `docs/FORMAT_EH3D.md`.

use std::borrow::BorrowMut;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use eh3d_container::{to_canonical_file_bytes, ContainerError, PkgCode, Violation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::appbranch::Notice;
use crate::attachfile::{
    danger_of, danger_of_file, sanitize_file_name, AttachmentKind, Danger, HEAD_BYTES,
};
use crate::meta::{check_text, LocalText};
use crate::project::{OpenOptions, Project, SaveOptions, StreamedFile, StreamedUpdate, Writer};
use crate::records::to_hex;
use crate::spec::{eh3d, is_sha256_hex, parse_major_minor, ATTACHMENTS_BRANCH, BRANCH_FILE};
use crate::suitetext::S;

/// The format of the branch `attachments`.
pub const FORMAT: &str = "1.0";
/// The record of the attachments inside their branch.
pub const INDEX_FILE: &str = "index.json";
/// The folder of the branch that holds the files.
pub const BLOB_DIR: &str = "blobs/";
/// The largest attachment by default: 1 GiB. A program may change it ([`AttachmentLimits`]); the whole project stays under the
/// limit of the container (4 GiB).
pub const DEFAULT_MAX_ATTACHMENT_BYTES: u64 = 1024 * 1024 * 1024;
/// The size of one stored part by default.
pub const DEFAULT_PART_BYTES: u64 = 32 * 1024 * 1024;
/// A part is never larger than the largest entry a reader of the format accepts in this branch.
const MAX_PART_BYTES: u64 = 256 * 1024 * 1024;
/// What is left free under the limit of the whole project for the records and for what the archive adds.
const TOTAL_HEADROOM: u64 = 16 * 1024 * 1024;
const MAX_ATTACHMENTS: usize = 10_000;
const MAX_LINKS: usize = 10_000;
const MAX_TEXT: usize = 256;
const MAX_URL: usize = 2048;
/// The shortest beginning of an identifier that names an attachment.
pub const MIN_ID_PREFIX: usize = 8;

/// Why something with attachments could not be done.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AttachError {
    /// The container refused or the file system failed.
    #[error("{0}")]
    Container(#[from] ContainerError),
    /// Something about a file on the computer.
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
    /// The file is larger than the limit of one attachment.
    #[error("{name}: {size} bytes, the limit of an attachment is {limit} bytes")]
    TooLarge {
        /// The name.
        name: String,
        /// Its size.
        size: u64,
        /// The limit.
        limit: u64,
    },
    /// The project would pass the limit of the whole container.
    #[error("{name}: the project would be larger than the {limit} bytes a project may hold")]
    ProjectFull {
        /// The name.
        name: String,
        /// The limit of the project.
        limit: u64,
    },
    /// The file is dangerous and nobody has agreed to take it out.
    #[error("{name} is marked dangerous ({}): it is taken out only when a person agrees", .danger.name())]
    NeedsConfirmation {
        /// The name.
        name: String,
        /// Why.
        danger: Danger,
    },
    /// The attachment is only a link: there is no file in the project.
    #[error("{name} is only a link ({url}): nothing is downloaded by itself")]
    LinkOnly {
        /// The name.
        name: String,
        /// The address.
        url: String,
    },
    /// No attachment has that identifier.
    #[error("there is no attachment {0}")]
    NotFound(String),
}

/// The result of this module.
pub type AttachResult<T> = std::result::Result<T, AttachError>;

fn refused(why: impl Into<String>) -> AttachError {
    AttachError::Refused(why.into())
}

fn in_file(path: &Path, why: impl ToString) -> AttachError {
    AttachError::File {
        file: path.display().to_string(),
        why: why.to_string(),
    }
}

/// The limits of attachments: plain data a program may take from its settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttachmentLimits {
    /// The largest file that is attached or taken out.
    pub max_attachment_bytes: u64,
    /// The size of the parts a large file is stored in (at most 256 MiB).
    pub part_bytes: u64,
}

impl Default for AttachmentLimits {
    fn default() -> AttachmentLimits {
        AttachmentLimits {
            max_attachment_bytes: DEFAULT_MAX_ATTACHMENT_BYTES,
            part_bytes: DEFAULT_PART_BYTES,
        }
    }
}

impl AttachmentLimits {
    /// Takes one line of a settings file. The key is `attachments.max_mb`, a whole number from 1 to 4096. Returns false for
    /// another key or a value that is not that; the limit then stays as it was.
    pub fn apply(&mut self, key: &str, value: &str) -> bool {
        if key.trim() != "attachments.max_mb" {
            return false;
        }
        match value.trim().parse::<u64>() {
            Ok(mb) if (1..=4096).contains(&mb) => {
                self.max_attachment_bytes = mb * 1024 * 1024;
                true
            }
            _ => false,
        }
    }

    /// With parts of this size (from 1 byte to 256 MiB).
    #[must_use]
    pub fn with_part_bytes(mut self, part_bytes: u64) -> AttachmentLimits {
        self.part_bytes = part_bytes;
        self
    }

    fn part(&self) -> u64 {
        self.part_bytes.clamp(1, MAX_PART_BYTES)
    }
}

/// What a piece of a design is for an attachment: **one** of a designator (`R5`), a value (`10k`) or a part number, the last one
/// with or without its manufacturer. Make one with [`ComponentLink::designator`], [`ComponentLink::of_value`] or
/// [`ComponentLink::part`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentLink {
    /// The designator of one piece of the design, compared without regard to case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The value, compared exactly (`1m` and `1M` are not the same).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The part number of the manufacturer, compared without regard to case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    /// The manufacturer of that part number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ComponentLink {
    /// The piece with this designator.
    pub fn designator(reference: &str) -> ComponentLink {
        ComponentLink {
            reference: Some(reference.trim().to_owned()),
            ..ComponentLink::default()
        }
    }

    /// Every piece with this value.
    pub fn of_value(value: &str) -> ComponentLink {
        ComponentLink {
            value: Some(value.trim().to_owned()),
            ..ComponentLink::default()
        }
    }

    /// Every piece that is this part.
    pub fn part(manufacturer: Option<&str>, part_number: &str) -> ComponentLink {
        ComponentLink {
            part_number: Some(part_number.trim().to_owned()),
            manufacturer: manufacturer.map(|m| m.trim().to_owned()),
            ..ComponentLink::default()
        }
    }

    fn validate(&self) -> Result<(), String> {
        let fields = [
            &self.reference,
            &self.value,
            &self.part_number,
            &self.manufacturer,
        ];
        for text in fields.into_iter().flatten() {
            if text.trim().is_empty() || text.len() > MAX_TEXT || text.chars().any(char::is_control)
            {
                return Err(
                    "a link to a component has an empty, too long or unreadable text".into(),
                );
            }
        }
        let forms = [&self.reference, &self.value, &self.part_number]
            .iter()
            .filter(|f| f.is_some())
            .count();
        // a form that a newer version added is kept as it is
        let unknown = forms == 0 && self.manufacturer.is_none() && !self.extra.is_empty();
        if !unknown && forms != 1 {
            return Err("a link to a component names one designator, value or part number".into());
        }
        if self.manufacturer.is_some() && self.part_number.is_none() {
            return Err("a manufacturer goes with a part number".into());
        }
        Ok(())
    }

    /// How well the link answers the question: the lower the more specific, or nothing when it does not.
    fn rank(&self, query: &ComponentQuery<'_>) -> Option<u8> {
        let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
        if let (Some(mine), Some(asked)) = (&self.reference, query.reference) {
            return same(mine, asked).then_some(0);
        }
        if let (Some(mine), Some(asked)) = (&self.part_number, query.part_number) {
            if !same(mine, asked) {
                return None;
            }
            return match (&self.manufacturer, query.manufacturer) {
                (Some(a), Some(b)) => same(a, b).then_some(1),
                _ => Some(2),
            };
        }
        match (&self.value, query.value) {
            (Some(mine), Some(asked)) => (mine.trim() == asked.trim()).then_some(3),
            _ => None,
        }
    }
}

/// What is known about the piece a person points at, to find its attachments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComponentQuery<'a> {
    /// Its designator.
    pub reference: Option<&'a str>,
    /// Its value.
    pub value: Option<&'a str>,
    /// Its part number.
    pub part_number: Option<&'a str>,
    /// Its manufacturer.
    pub manufacturer: Option<&'a str>,
}

impl<'a> ComponentQuery<'a> {
    /// A question with nothing in it.
    pub fn new() -> ComponentQuery<'a> {
        ComponentQuery::default()
    }

    /// With the designator.
    #[must_use]
    pub fn reference(mut self, reference: &'a str) -> Self {
        self.reference = Some(reference);
        self
    }

    /// With the value.
    #[must_use]
    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// With the part number.
    #[must_use]
    pub fn part_number(mut self, part_number: &'a str) -> Self {
        self.part_number = Some(part_number);
        self
    }

    /// With the manufacturer.
    #[must_use]
    pub fn manufacturer(mut self, manufacturer: &'a str) -> Self {
        self.manufacturer = Some(manufacturer);
        self
    }
}

/// Where the file of an attachment is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    /// In the project.
    Inside,
    /// Somewhere else: the record keeps the address and the hash.
    Link,
}

/// One attachment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Attachment {
    /// The SHA-256 of the file, which is also the identifier of the attachment.
    pub sha256: String,
    /// The file name, safe for any system.
    pub name: String,
    /// The size in bytes; a link may not know it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// What it is, as a word of [`AttachmentKind`].
    pub kind: String,
    /// Why it is dangerous, as a word of [`Danger`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danger: Option<String>,
    /// Inside the project or only a link.
    pub storage: Storage,
    /// The address of a link, or where the file came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A title in the languages of the project.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub title: LocalText,
    /// When it was attached, as the caller wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<String>,
    /// The pieces of the design it belongs to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ComponentLink>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Attachment {
    /// The identifier.
    pub fn id(&self) -> &str {
        &self.sha256
    }

    /// The kind.
    pub fn kind(&self) -> AttachmentKind {
        AttachmentKind::parse(&self.kind)
    }

    /// Why it is dangerous: what the record says and what the name says, whichever warns.
    pub fn danger(&self) -> Option<Danger> {
        self.danger
            .as_deref()
            .map(Danger::parse)
            .or_else(|| danger_of(&self.name, &[]))
    }

    /// True when a person has to agree before it is taken out or opened.
    pub fn needs_confirmation(&self) -> bool {
        self.danger().is_some()
    }

    /// The name to give to a file taken out of the project: cleaned again, whatever the record says.
    pub fn file_name(&self) -> String {
        sanitize_file_name(&self.name)
    }

    /// True when the project holds only its address and hash.
    pub fn is_link(&self) -> bool {
        self.storage == Storage::Link
    }

    fn blob_dir(&self) -> String {
        blob_dir(&self.sha256)
    }
}

fn blob_dir(sha256: &str) -> String {
    format!("{BLOB_DIR}{sha256}/")
}

fn check_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let plain = url.len() <= MAX_URL && !url.chars().any(|c| c.is_control() || c.is_whitespace());
    let host = rest.map(|r| r.split(['/', '?', '#']).next().unwrap_or(""));
    match host {
        Some(host) if plain && !host.is_empty() && !host.contains('@') => Ok(()),
        _ => Err(
            "an address starts with https:// or http:// and has no name or password in it".into(),
        ),
    }
}

/// What `attachments/index.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentIndex {
    /// The schema number.
    pub schema: u32,
    /// The attachments, by their identifier.
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    /// What this crate does not know.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for AttachmentIndex {
    fn default() -> AttachmentIndex {
        AttachmentIndex {
            schema: 1,
            attachments: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

impl AttachmentIndex {
    /// The attachment of an identifier, or of the only identifier that starts with it ([`MIN_ID_PREFIX`] digits at least).
    pub fn find(&self, id: &str) -> AttachResult<&Attachment> {
        let wanted = id.trim().to_ascii_lowercase();
        if wanted.len() < MIN_ID_PREFIX {
            return Err(refused(format!(
                "an attachment is named by {MIN_ID_PREFIX} digits of its identifier at least"
            )));
        }
        let mut found = self
            .attachments
            .iter()
            .filter(|a| a.sha256.starts_with(&wanted));
        match (found.next(), found.next()) {
            (Some(one), None) => Ok(one),
            (Some(_), Some(_)) => Err(refused(format!("{id} names more than one attachment"))),
            _ => Err(AttachError::NotFound(id.to_owned())),
        }
    }

    /// The attachments of a piece of the design, the most specific first: the ones of its designator, then of its part number with
    /// the manufacturer, then of its part number, then of its value.
    pub fn for_component(&self, query: &ComponentQuery<'_>) -> Vec<&Attachment> {
        let mut found: Vec<(u8, &Attachment)> = self
            .attachments
            .iter()
            .filter_map(|a| {
                a.components
                    .iter()
                    .filter_map(|link| link.rank(query))
                    .min()
                    .map(|rank| (rank, a))
            })
            .collect();
        found.sort_by(|a, b| (a.0, &a.1.name, &a.1.sha256).cmp(&(b.0, &b.1.name, &b.1.sha256)));
        found.into_iter().map(|(_, a)| a).collect()
    }

    /// Checks the index against the files of its branch (name, size, hash): the names are safe, an identifier is a SHA-256 listed
    /// once, a link has an address, and the parts of a file inside are there with the size announced.
    pub fn validate<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, u64, &'a str)>,
    ) -> Result<(), String> {
        let files: Vec<(&str, u64, &str)> = files.into_iter().collect();
        if self.schema != 1 {
            return Err("the schema of the index is not known".into());
        }
        if self.attachments.len() > MAX_ATTACHMENTS {
            return Err("there are too many attachments".into());
        }
        let mut seen = BTreeSet::new();
        for a in &self.attachments {
            let name = &a.name;
            if !is_sha256_hex(&a.sha256) {
                return Err(format!("'{name}': the identifier is not a SHA-256"));
            }
            if !seen.insert(a.sha256.as_str()) {
                return Err(format!("'{name}' is listed twice"));
            }
            if *name != a.file_name() {
                return Err(format!("'{}' is not a safe name", a.file_name()));
            }
            check_text(name, &a.title)?;
            if a.components.len() > MAX_LINKS {
                return Err(format!("'{name}' has too many components"));
            }
            for link in &a.components {
                link.validate().map_err(|why| format!("'{name}': {why}"))?;
            }
            if let Some(url) = &a.url {
                check_url(url).map_err(|why| format!("'{name}': {why}"))?;
            }
            match a.storage {
                Storage::Link => {
                    if a.url.is_none() {
                        return Err(format!("'{name}' is a link without an address"));
                    }
                }
                Storage::Inside => {
                    let dir = a.blob_dir();
                    let parts: Vec<&(&str, u64, &str)> =
                        files.iter().filter(|f| f.0.starts_with(&dir)).collect();
                    let total: u64 = parts.iter().map(|p| p.1).sum();
                    if parts.is_empty() || a.size != Some(total) {
                        return Err(format!("'{name}': the stored file is not complete"));
                    }
                    if parts.len() == 1 && parts[0].2 != a.sha256 {
                        return Err(format!(
                            "'{name}': the stored file is not the one announced"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// The attachments of an open project; none when it has no branch of attachments. A branch of a newer format is refused (024).
pub fn read_index(project: &mut Project) -> AttachResult<AttachmentIndex> {
    let Some(branch) = project.branch(ATTACHMENTS_BRANCH) else {
        return Ok(AttachmentIndex::default());
    };
    if let Some(record) = &branch.record {
        if parse_major_minor(&record.format).is_none_or(|(major, minor)| major != 1 || minor > 0) {
            return Err(ContainerError::Rejected(
                Violation::new(
                    PkgCode::BranchVersion,
                    format!("the attachments have the format {}", record.format),
                )
                .at(ATTACHMENTS_BRANCH),
            )
            .into());
        }
    }
    if !branch.files.contains_key(INDEX_FILE) {
        return Err(refused(
            "the branch attachments has no index: it was not written as attachments",
        ));
    }
    let bytes = project.read_entry(ATTACHMENTS_BRANCH, INDEX_FILE)?;
    serde_json::from_slice(&bytes).map_err(|e| {
        refused(format!(
            "{ATTACHMENTS_BRANCH}/{INDEX_FILE} is not a valid record: {e}"
        ))
    })
}

/// Whether a person has agreed to take out or open a dangerous file. An agreement is good for one action.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Consent {
    /// Nobody has been asked.
    #[default]
    NotAsked,
    /// The person was told why it is dangerous and agreed.
    Given,
}

/// What goes with a file or a link when it is attached.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AttachRequest {
    /// Another name than the one of the file.
    pub name: Option<String>,
    /// A title in the languages of the project.
    pub title: LocalText,
    /// The pieces of the design it belongs to.
    pub components: Vec<ComponentLink>,
    /// Where the file came from.
    pub url: Option<String>,
    /// When, as the caller writes it (this crate never reads the clock).
    pub added: Option<String>,
}

impl AttachRequest {
    /// A request with nothing but the file.
    pub fn new() -> AttachRequest {
        AttachRequest::default()
    }

    /// With another name.
    #[must_use]
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// With a title in one language.
    #[must_use]
    pub fn title(mut self, lang: &str, text: &str) -> Self {
        self.title.insert(lang.to_owned(), text.to_owned());
        self
    }

    /// With a piece of the design.
    #[must_use]
    pub fn component(mut self, link: ComponentLink) -> Self {
        self.components.push(link);
        self
    }

    /// With the address it came from.
    #[must_use]
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_owned());
        self
    }

    fn validate(&self) -> AttachResult<()> {
        check_text("title", &self.title).map_err(refused)?;
        for link in &self.components {
            link.validate().map_err(refused)?;
        }
        if let Some(url) = &self.url {
            check_url(url).map_err(refused)?;
        }
        Ok(())
    }
}

/// What a write of the attachments did.
pub struct Attached {
    /// The project, opened again from the new file.
    pub project: Project,
    /// The identifier of the attachment.
    pub id: String,
    /// The same content was attached before: it is stored once and only what is new about it was added.
    pub already_there: bool,
    /// False when there was nothing new and the file was not written.
    pub written: bool,
    /// What the person should be told.
    pub notices: Vec<Notice>,
}

impl std::fmt::Debug for Attached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attached")
            .field("id", &self.id)
            .field("already_there", &self.already_there)
            .field("written", &self.written)
            .field("notices", &self.notices)
            .finish_non_exhaustive()
    }
}

/// Merges what a request says into a record that is there.
fn merge(record: &mut Attachment, request: &AttachRequest) {
    for link in &request.components {
        if !record.components.contains(link) {
            record.components.push(link.clone());
        }
    }
    for (lang, text) in &request.title {
        record.title.insert(lang.clone(), text.clone());
    }
    if request.url.is_some() {
        record.url.clone_from(&request.url);
    }
}

/// The size, the hash, the hashes of the parts and the first bytes of a file, read once in blocks.
struct Measured {
    sha256: String,
    size: u64,
    parts: Vec<(u64, String)>,
    head: Vec<u8>,
}

fn measure(path: &Path, name: &str, limits: &AttachmentLimits) -> AttachResult<Measured> {
    let mut file = File::open(path).map_err(|e| in_file(path, e))?;
    let limit = limits.max_attachment_bytes;
    let part = limits.part();
    let mut whole = Sha256::new();
    let mut current = Sha256::new();
    let mut in_part = 0u64;
    let mut size = 0u64;
    let mut parts = Vec::new();
    let mut head = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| in_file(path, e))?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > limit {
            return Err(AttachError::TooLarge {
                name: name.to_owned(),
                size,
                limit,
            });
        }
        let mut block = &buffer[..n];
        if head.len() < HEAD_BYTES {
            head.extend_from_slice(&block[..block.len().min(HEAD_BYTES - head.len())]);
        }
        whole.update(block);
        while !block.is_empty() {
            let room = usize::try_from(part - in_part).unwrap_or(usize::MAX);
            let take = block.len().min(room);
            current.update(&block[..take]);
            in_part += take as u64;
            block = &block[take..];
            if in_part == part {
                parts.push((in_part, to_hex(&current.finalize_reset())));
                in_part = 0;
            }
        }
    }
    if in_part > 0 || parts.is_empty() {
        parts.push((in_part, to_hex(&current.finalize_reset())));
    }
    Ok(Measured {
        sha256: to_hex(&whole.finalize()),
        size,
        parts,
        head,
    })
}

/// A program that writes attachments: its name and version, as the record of the branch says them, and its limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Attacher {
    /// The program.
    pub app: String,
    /// Its version.
    pub app_version: String,
    /// The limits.
    pub limits: AttachmentLimits,
}

impl Attacher {
    /// A writer with the limits by default.
    pub fn new(app: &str, app_version: &str) -> Attacher {
        Attacher {
            app: app.to_owned(),
            app_version: app_version.to_owned(),
            limits: AttachmentLimits::default(),
        }
    }

    /// With these limits.
    #[must_use]
    pub fn with_limits(mut self, limits: AttachmentLimits) -> Attacher {
        self.limits = limits;
        self
    }

    fn writer(&self) -> Writer {
        Writer::new(&self.app, &[])
    }

    /// Refuses before any file is read when the project or the branch cannot be written.
    fn check(&self, project: &mut Project) -> AttachResult<AttachmentIndex> {
        project.check_may_write(
            &self.writer(),
            ATTACHMENTS_BRANCH,
            FORMAT,
            &self.app_version,
        )?;
        read_index(project)
    }

    /// Puts the file at `source` into the project. The file is read in blocks. A content that is there already is not stored
    /// again: its components, title and address are added to it (and a link becomes a file inside).
    pub fn attach_file(
        &self,
        mut project: Project,
        source: &Path,
        request: &AttachRequest,
        options: &SaveOptions<'_>,
    ) -> AttachResult<Attached> {
        let mut index = self.check(&mut project)?;
        request.validate()?;
        let meta = fs::metadata(source).map_err(|e| in_file(source, e))?;
        if !meta.is_file() {
            return Err(in_file(source, "it is not a file"));
        }
        let given = match &request.name {
            Some(name) => name.clone(),
            None => crate::appbranch::file_name(source).map_err(|e| in_file(source, e))?,
        };
        let name = sanitize_file_name(&given);
        let limit = self.limits.max_attachment_bytes;
        if meta.len() > limit {
            return Err(AttachError::TooLarge {
                name,
                size: meta.len(),
                limit,
            });
        }
        let measured = measure(source, &name, &self.limits)?;
        if crate::ed25519::looks_like_secret_key(&measured.head) {
            return Err(refused("a secret key is never put into a project"));
        }
        let mut file = File::open(source).map_err(|e| in_file(source, e))?;
        // the name of the file on disk and the name it gets both count
        let mut danger = danger_of_file(&name, &mut file).map_err(|e| in_file(source, e))?;
        if danger.is_none() {
            let on_disk = crate::appbranch::file_name(source).map_err(|e| in_file(source, e))?;
            danger = danger_of(&on_disk, &measured.head);
        }

        let before = index.clone();
        let found = index
            .attachments
            .iter()
            .position(|a| a.sha256 == measured.sha256);
        let stored = found.is_some_and(|i| !index.attachments[i].is_link());
        let mut streams = Vec::new();
        if !stored {
            let total_limit = project.spec().limits.max_total_bytes;
            let held: u64 = project
                .branches()
                .flat_map(|b| b.files.values())
                .map(|f| f.size)
                .sum();
            if held
                .saturating_add(measured.size)
                .saturating_add(TOTAL_HEADROOM)
                > total_limit
            {
                return Err(AttachError::ProjectFull {
                    name,
                    limit: total_limit,
                });
            }
            let dir = blob_dir(&measured.sha256);
            let mut offset = 0;
            for (n, (size, sha256)) in measured.parts.iter().enumerate() {
                streams.push(StreamedFile {
                    rel: format!("{dir}{n:05}"),
                    path: source.to_path_buf(),
                    offset,
                    size: *size,
                    sha256: sha256.clone(),
                });
                offset += size;
            }
        }
        match found {
            Some(i) => {
                let record = &mut index.attachments[i];
                merge(record, request);
                record.storage = Storage::Inside;
                record.size = Some(measured.size);
                if record.danger.is_none() {
                    record.danger = danger.map(|d| d.name().to_owned());
                }
            }
            None => {
                if index.attachments.len() >= MAX_ATTACHMENTS {
                    return Err(refused("there are too many attachments"));
                }
                let mut record = Attachment {
                    sha256: measured.sha256.clone(),
                    kind: AttachmentKind::of_name(&name).name().to_owned(),
                    name,
                    size: Some(measured.size),
                    danger: danger.map(|d| d.name().to_owned()),
                    storage: Storage::Inside,
                    url: None,
                    title: LocalText::new(),
                    added: request.added.clone(),
                    components: Vec::new(),
                    extra: BTreeMap::new(),
                };
                merge(&mut record, request);
                index.attachments.push(record);
            }
        }
        self.write(
            project,
            Change {
                before,
                index,
                id: measured.sha256,
                already_there: found.is_some(),
                streams,
                removed: None,
            },
            options,
        )
    }

    /// Keeps only the address and the hash of a file. Nothing is downloaded. `size` is the size when it is known. The name is the
    /// one of the request, or the last part of the address.
    pub fn attach_link(
        &self,
        mut project: Project,
        url: &str,
        sha256: &str,
        size: Option<u64>,
        request: &AttachRequest,
        options: &SaveOptions<'_>,
    ) -> AttachResult<Attached> {
        let mut index = self.check(&mut project)?;
        let request = request.clone().url(url);
        request.validate()?;
        let sha256 = sha256.trim().to_ascii_lowercase();
        if !is_sha256_hex(&sha256) {
            return Err(refused(
                "a link needs the SHA-256 of the file it must give (64 hexadecimal digits)",
            ));
        }
        let from_url = url.split(['?', '#']).next().unwrap_or(url);
        let name = sanitize_file_name(
            request
                .name
                .as_deref()
                .unwrap_or_else(|| from_url.trim_end_matches('/')),
        );
        let before = index.clone();
        let found = index.attachments.iter().position(|a| a.sha256 == sha256);
        match found {
            Some(i) => merge(&mut index.attachments[i], &request),
            None => {
                if index.attachments.len() >= MAX_ATTACHMENTS {
                    return Err(refused("there are too many attachments"));
                }
                let mut record = Attachment {
                    sha256: sha256.clone(),
                    kind: AttachmentKind::of_name(&name).name().to_owned(),
                    danger: danger_of(&name, &[]).map(|d| d.name().to_owned()),
                    name,
                    size,
                    storage: Storage::Link,
                    url: None,
                    title: LocalText::new(),
                    added: request.added.clone(),
                    components: Vec::new(),
                    extra: BTreeMap::new(),
                };
                merge(&mut record, &request);
                index.attachments.push(record);
            }
        }
        self.write(
            project,
            Change {
                before,
                index,
                id: sha256,
                already_there: found.is_some(),
                streams: Vec::new(),
                removed: None,
            },
            options,
        )
    }

    /// Adds and takes away pieces of the design of one attachment.
    pub fn set_components(
        &self,
        mut project: Project,
        id: &str,
        add: &[ComponentLink],
        remove: &[ComponentLink],
        options: &SaveOptions<'_>,
    ) -> AttachResult<Attached> {
        let mut index = self.check(&mut project)?;
        for link in add {
            link.validate().map_err(refused)?;
        }
        let id = index.find(id)?.sha256.clone();
        let before = index.clone();
        for record in index.attachments.iter_mut().filter(|a| a.sha256 == id) {
            record.components.retain(|c| !remove.contains(c));
            for link in add {
                if !record.components.contains(link) {
                    record.components.push(link.clone());
                }
            }
        }
        self.write(
            project,
            Change {
                before,
                index,
                id,
                already_there: true,
                streams: Vec::new(),
                removed: None,
            },
            options,
        )
    }

    /// Takes an attachment out of the project for good, with its file.
    pub fn detach(
        &self,
        mut project: Project,
        id: &str,
        options: &SaveOptions<'_>,
    ) -> AttachResult<Attached> {
        let mut index = self.check(&mut project)?;
        let id = index.find(id)?.sha256.clone();
        let before = index.clone();
        index.attachments.retain(|a| a.sha256 != id);
        self.write(
            project,
            Change {
                before,
                index,
                already_there: true,
                streams: Vec::new(),
                removed: Some(blob_dir(&id)),
                id,
            },
            options,
        )
    }

    fn write(
        &self,
        project: Project,
        mut change: Change,
        options: &SaveOptions<'_>,
    ) -> AttachResult<Attached> {
        change
            .index
            .attachments
            .sort_by(|a, b| a.sha256.cmp(&b.sha256));
        if change.index == change.before && change.streams.is_empty() {
            return Ok(Attached {
                project,
                id: change.id,
                already_there: change.already_there,
                written: false,
                notices: Vec::new(),
            });
        }
        let old = project.branch(ATTACHMENTS_BRANCH);
        let mut notices = Vec::new();
        if project.record().signature.is_some() {
            notices.push(Notice::ProjectSignatureRemoved);
        }
        if options.signer.is_none()
            && old.is_some_and(|b| b.record.as_ref().is_some_and(|r| r.signature.is_some()))
        {
            notices.push(Notice::BranchSignatureRemoved);
        }
        let keep: BTreeSet<String> = old
            .map(|b| {
                b.files
                    .keys()
                    .filter(|rel| rel.as_str() != BRANCH_FILE && rel.as_str() != INDEX_FILE)
                    .filter(|rel| {
                        !change
                            .removed
                            .as_ref()
                            .is_some_and(|dir| rel.starts_with(dir.as_str()))
                    })
                    // a part that is written again replaces the one that was there
                    .filter(|rel| !change.streams.iter().any(|s| &s.rel == *rel))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let extra = old
            .and_then(|b| b.record.as_ref())
            .map(|r| r.extra.clone())
            .unwrap_or_default();
        let bytes = to_canonical_file_bytes(&change.index)
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))?;
        let update = StreamedUpdate {
            id: ATTACHMENTS_BRANCH.to_owned(),
            format: FORMAT.to_owned(),
            app_version: self.app_version.clone(),
            keep,
            entries: BTreeMap::from([(INDEX_FILE.to_owned(), bytes)]),
            files: change.streams,
            extra,
        };
        let project = project.save_streamed(&self.writer(), update, options)?;
        Ok(Attached {
            project,
            id: change.id,
            already_there: change.already_there,
            written: true,
            notices,
        })
    }
}

struct Change {
    before: AttachmentIndex,
    index: AttachmentIndex,
    id: String,
    already_there: bool,
    streams: Vec<StreamedFile>,
    removed: Option<String>,
}

/// The bytes of one attachment, read part by part (at most one part is in memory). The size and the SHA-256 are checked before
/// the last part is handed over: a file that is not the one announced ends in an error.
pub struct AttachmentReader<P: BorrowMut<Project> = Project> {
    project: P,
    parts: Vec<String>,
    next: usize,
    buffer: Vec<u8>,
    at: usize,
    hasher: Sha256,
    read: u64,
    size: u64,
    sha256: String,
    failed: bool,
}

fn damaged(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_owned())
}

impl<P: BorrowMut<Project>> AttachmentReader<P> {
    fn new(mut project: P, id: &str) -> AttachResult<AttachmentReader<P>> {
        let inner: &mut Project = project.borrow_mut();
        let index = read_index(inner)?;
        let record = index.find(id)?;
        if record.is_link() {
            return Err(AttachError::LinkOnly {
                name: record.file_name(),
                url: record.url.clone().unwrap_or_default(),
            });
        }
        let dir = record.blob_dir();
        let mut parts = Vec::new();
        let mut total = 0u64;
        if let Some(branch) = inner.branch(ATTACHMENTS_BRANCH) {
            for (rel, info) in &branch.files {
                if rel.starts_with(&dir) {
                    parts.push(rel.clone());
                    total = total.saturating_add(info.size);
                }
            }
        }
        if parts.is_empty() || record.size != Some(total) {
            return Err(refused(format!(
                "{}: the stored file is not complete",
                record.file_name()
            )));
        }
        let sha256 = record.sha256.clone();
        Ok(AttachmentReader {
            project,
            parts,
            next: 0,
            buffer: Vec::new(),
            at: 0,
            hasher: Sha256::new(),
            read: 0,
            size: total,
            sha256,
            failed: false,
        })
    }

    fn load(&mut self) -> io::Result<()> {
        let project: &mut Project = self.project.borrow_mut();
        let bytes = project
            .read_entry(ATTACHMENTS_BRANCH, &self.parts[self.next])
            .map_err(io::Error::other)?;
        self.next += 1;
        self.hasher.update(&bytes);
        self.read = self.read.saturating_add(bytes.len() as u64);
        if self.next == self.parts.len()
            && (self.read != self.size || to_hex(&self.hasher.clone().finalize()) != self.sha256)
        {
            return Err(damaged(
                "the stored file is not the one the project announces",
            ));
        }
        self.buffer = bytes;
        self.at = 0;
        Ok(())
    }
}

impl<P: BorrowMut<Project>> Read for AttachmentReader<P> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(damaged("the attachment could not be read"));
        }
        while self.at == self.buffer.len() {
            if self.next == self.parts.len() {
                return Ok(0);
            }
            if let Err(e) = self.load() {
                self.failed = true;
                return Err(e);
            }
        }
        let n = out.len().min(self.buffer.len() - self.at);
        out[..n].copy_from_slice(&self.buffer[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// A stream of the bytes of one attachment of an open project. Reading is not taking out: a program that shows a dangerous file
/// asks the person first ([`Attachment::needs_confirmation`]) and never runs it.
pub fn reader<'a>(
    project: &'a mut Project,
    id: &str,
) -> AttachResult<AttachmentReader<&'a mut Project>> {
    AttachmentReader::new(project, id)
}

/// One attachment of a project file as a thing that can be read as often as needed, for a viewer: a name, a length and a reader
/// from the first byte. It holds no open file; it can be sent between threads.
#[derive(Debug, Clone)]
pub struct AttachmentSource {
    path: PathBuf,
    record: Attachment,
}

/// Opening for one attachment does not hash the whole project: the content is checked against its own SHA-256 while it is read.
fn open_to_read(path: &Path) -> AttachResult<Project> {
    Ok(Project::open(
        path,
        &OpenOptions {
            spec: eh3d(),
            verify_integrity: false,
        },
    )?)
}

impl AttachmentSource {
    /// The attachment `id` of the project at `path`. A link has no bytes and is refused.
    pub fn open(path: &Path, id: &str) -> AttachResult<AttachmentSource> {
        let mut project = open_to_read(path)?;
        let index = read_index(&mut project)?;
        let record = index.find(id)?;
        if record.is_link() {
            return Err(AttachError::LinkOnly {
                name: record.file_name(),
                url: record.url.clone().unwrap_or_default(),
            });
        }
        let mut record = record.clone();
        record.name = record.file_name();
        Ok(AttachmentSource {
            path: path.to_path_buf(),
            record,
        })
    }

    /// The record.
    pub fn record(&self) -> &Attachment {
        &self.record
    }

    /// The safe file name.
    pub fn name(&self) -> &str {
        &self.record.name
    }

    /// The size announced.
    pub fn len(&self) -> u64 {
        self.record.size.unwrap_or(0)
    }

    /// True when nothing is announced.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A reader from the first byte.
    pub fn reader(&self) -> io::Result<AttachmentReader> {
        open_to_read(&self.path)
            .and_then(|project| AttachmentReader::new(project, &self.record.sha256))
            .map_err(io::Error::other)
    }
}

/// Marks a file as coming from outside, so Windows and the office programs treat it with care.
fn mark_as_foreign(path: &Path) {
    if cfg!(windows) {
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        // a file system without streams has no mark; there is nothing else to do
        let _ = fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n");
    }
}

fn copy_out(
    from: &mut impl Read,
    file: &mut File,
    name: &str,
    consent: Consent,
) -> AttachResult<()> {
    io::copy(from, file).map_err(|e| refused(format!("{name}: {e}")))?;
    file.flush().map_err(|e| refused(format!("{name}: {e}")))?;
    if consent == Consent::NotAsked {
        // the record may not say what the content is: look at the file itself
        if let Some(danger) =
            danger_of_file(name, file).map_err(|e| refused(format!("{name}: {e}")))?
        {
            return Err(AttachError::NeedsConfirmation {
                name: name.to_owned(),
                danger,
            });
        }
    }
    Ok(())
}

/// Takes one attachment out into `directory` under its safe name, never over a file that is there. A dangerous file needs
/// [`Consent::Given`]. The file is not executable, it is marked on Windows as coming from outside, it is checked against its
/// SHA-256 and a partial or refused one is removed. Nothing is opened or run.
pub fn extract(
    project: &mut Project,
    id: &str,
    directory: &Path,
    consent: Consent,
    limits: &AttachmentLimits,
) -> AttachResult<PathBuf> {
    let index = read_index(project)?;
    let record = index.find(id)?;
    let name = record.file_name();
    if record.is_link() {
        return Err(AttachError::LinkOnly {
            name,
            url: record.url.clone().unwrap_or_default(),
        });
    }
    if let (Some(danger), Consent::NotAsked) = (record.danger(), consent) {
        return Err(AttachError::NeedsConfirmation { name, danger });
    }
    let size = record.size.unwrap_or(0);
    if size > limits.max_attachment_bytes {
        return Err(AttachError::TooLarge {
            name,
            size,
            limit: limits.max_attachment_bytes,
        });
    }
    let sha256 = record.sha256.clone();
    let mut from = AttachmentReader::new(project, &sha256)?;
    fs::create_dir_all(directory).map_err(|e| in_file(directory, e))?;
    let target = directory.join(&name);
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&target).map_err(|e| in_file(&target, e))?;
    let outcome = copy_out(&mut from, &mut file, &name, consent);
    drop(file);
    if let Err(e) = outcome {
        let _ = fs::remove_file(&target);
        return Err(e);
    }
    mark_as_foreign(&target);
    Ok(target)
}

/// Whether the file at `path` (that a person got from the address of a link) is the one the attachment announces. Nothing is
/// downloaded.
pub fn matches_file(record: &Attachment, path: &Path) -> AttachResult<bool> {
    let limits = AttachmentLimits {
        max_attachment_bytes: u64::MAX,
        part_bytes: MAX_PART_BYTES,
    };
    let measured = measure(path, &record.name, &limits)?;
    Ok(measured.sha256 == record.sha256 && record.size.is_none_or(|s| s == measured.size))
}

/// The SHA-256 and the size of a file, read in blocks: what a link to it records.
pub fn hash_file(path: &Path) -> AttachResult<(String, u64)> {
    let limits = AttachmentLimits {
        max_attachment_bytes: u64::MAX,
        part_bytes: MAX_PART_BYTES,
    };
    let measured = measure(path, "file", &limits)?;
    Ok((measured.sha256, measured.size))
}

/// The word of the window (in the eight languages) for a danger.
pub fn danger_word(danger: Danger) -> S {
    match danger {
        Danger::Executable => S::DangerExecutable,
        Danger::Script => S::DangerScript,
        Danger::Macros => S::DangerMacros,
        Danger::Archive => S::DangerArchive,
        Danger::ActiveContent => S::DangerActive,
    }
}
