//! What the attachment panel holds and does, with no window in it.
//!
//! The panel lists the attachments of the project of the program. Everything that writes the project, or takes a file out of it,
//! is a [`Request`]; what a person has to agree to first becomes a [`Question`], and only [`Model::confirm`] goes on. The work runs
//! on a thread of its own, because a file may be a gigabyte: [`Model::poll`] collects the answer. The SHA-256 of a file is
//! taken on another thread for the same reason. Nothing here runs, opens or downloads anything of an attachment.
//!
//! A write is refused, with nothing written, when another program saved the project after the panel read it; the list is then
//! read again. What the program of the panel itself writes does not count (see [`eh3d_project::revision`]).

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::SystemTime;

use eh3d_container::PkgCode;
use eh3d_project::attachments::{self, read_index, AttachmentSource};
use eh3d_project::revision::{self, Revision};
use eh3d_project::spec::{eh3d, ATTACHMENTS_BRANCH};
use eh3d_project::suitetext::S;
use eh3d_project::{
    appbranch, suiteview, AttachError, AttachRequest, Attacher, Attachment, AttachmentIndex,
    AttachmentKind, AttachmentLimits, ComponentLink, ComponentQuery, Consent, Danger, OpenOptions,
    Project, SaveOptions,
};

use crate::texts::{fill, tr, Lang, Text};

/// A settings file larger than this is not looked into for credentials.
const MAX_SETTINGS_BYTES: u64 = 4 << 20;

/// The program the panel works for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The branch of its own (`laser`, `cnc`, `pnp`), which is also the name it writes with: what a write would unsign is
    /// asked about.
    pub branch: String,
    /// Its version.
    pub version: String,
}

impl Program {
    /// The program of a branch, in a version.
    pub fn new(branch: &str, version: &str) -> Program {
        Program {
            branch: branch.to_owned(),
            version: version.to_owned(),
        }
    }
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The identifier.
    pub id: String,
    /// The safe name.
    pub name: String,
    /// What it is.
    pub kind: AttachmentKind,
    /// Its size, when it is known.
    pub size: Option<u64>,
    /// Only an address and a hash.
    pub is_link: bool,
    /// The address of a link, or where the file came from.
    pub url: Option<String>,
    /// Why it is dangerous, when it is.
    pub danger: Option<Danger>,
    /// A person has to agree before it is taken out.
    pub needs_confirmation: bool,
    /// The components it belongs to.
    pub components: Vec<ComponentLink>,
}

impl Item {
    fn of(record: &Attachment) -> Item {
        Item {
            id: record.id().to_owned(),
            name: record.file_name(),
            kind: record.kind(),
            size: record.size,
            is_link: record.is_link(),
            url: record.url.clone(),
            danger: record.danger(),
            needs_confirmation: record.needs_confirmation(),
            components: record.components.clone(),
        }
    }
}

/// The piece of the design whose attachments are looked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Piece {
    /// Its designator.
    pub reference: Option<String>,
    /// Its value.
    pub value: Option<String>,
    /// Its part number.
    pub part_number: Option<String>,
    /// Its manufacturer.
    pub manufacturer: Option<String>,
}

impl Piece {
    fn query(&self) -> ComponentQuery<'_> {
        let mut query = ComponentQuery::new();
        if let Some(reference) = &self.reference {
            query = query.reference(reference);
        }
        if let Some(value) = &self.value {
            query = query.value(value);
        }
        if let Some(number) = &self.part_number {
            query = query.part_number(number);
        }
        if let Some(maker) = &self.manufacturer {
            query = query.manufacturer(maker);
        }
        query
    }

    /// The words that name the piece in the window.
    pub fn label(&self) -> String {
        [
            &self.reference,
            &self.value,
            &self.manufacturer,
            &self.part_number,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
    }
}

/// The form of a link to a component.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinkForm {
    /// One designator.
    #[default]
    Designator,
    /// A value.
    Value,
    /// A part number, with or without its manufacturer.
    PartNumber,
}

/// The link a form says, or nothing when its text is empty.
pub fn component_link(form: LinkForm, text: &str, maker: &str) -> Option<ComponentLink> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let maker = maker.trim();
    Some(match form {
        LinkForm::Designator => ComponentLink::designator(text),
        LinkForm::Value => ComponentLink::of_value(text),
        LinkForm::PartNumber => ComponentLink::part(Some(maker).filter(|m| !m.is_empty()), text),
    })
}

/// The words of a link to a component, for the list.
pub fn link_label(link: &ComponentLink) -> String {
    [
        &link.reference,
        &link.value,
        &link.manufacturer,
        &link.part_number,
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>()
    .join(" ")
}

/// Something a person asks the panel to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Put this file into the project.
    AttachFile(PathBuf),
    /// Keep only the address and the hash of a file.
    AttachLink {
        /// The address.
        url: String,
        /// The SHA-256 of the file it must give.
        sha256: String,
        /// Its size, when it is known.
        size: Option<u64>,
    },
    /// Write the file of an attachment into a folder.
    Extract(String, PathBuf),
    /// Take an attachment out of the project for good.
    Remove(String),
    /// Link an attachment to a component.
    AddComponent(String, ComponentLink),
    /// Remove a link to a component.
    RemoveComponent(String, ComponentLink),
}

/// A line of a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A file was dropped and the system did not say where: it is attached only after a yes.
    Dropped(String),
    /// The write removes the signature of the whole project.
    SignedProject,
    /// The write removes the signature of the branch of the attachments.
    SignedAttachments,
    /// The attachment is taken out of the project for good.
    Remove {
        /// Its name.
        name: String,
        /// How many components point at it.
        components: usize,
    },
    /// The file is dangerous, or of a kind the suite does not know.
    Care {
        /// Its name.
        name: String,
        /// Why it is dangerous; nothing for an unknown kind.
        danger: Option<Danger>,
    },
}

impl Ask {
    /// The words of the line.
    pub fn text(&self, lang: Lang) -> String {
        let code = lang.code();
        match self {
            Ask::Dropped(name) => fill(tr(lang, Text::AskDropped), &[name]),
            Ask::SignedProject => S::SignedProject.tr(code).to_owned(),
            Ask::SignedAttachments => fill(S::SignedBranch.tr(code), &[ATTACHMENTS_BRANCH]),
            Ask::Remove { name, components } => {
                let mut said = fill(tr(lang, Text::AskRemove), &[name]);
                if *components > 0 {
                    said.push(' ');
                    said.push_str(&fill(
                        tr(lang, Text::AskRemoveParts),
                        &[&components.to_string()],
                    ));
                }
                said
            }
            Ask::Care { name, danger } => fill(
                tr(lang, Text::AskExtract),
                &[name, attachments::doubt_word(*danger).tr(code)],
            ),
        }
    }
}

/// A request that waits for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// What was asked.
    pub request: Request,
    /// What has to be agreed to.
    pub lines: Vec<Ask>,
}

/// What is to be told about the last thing done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// A file or a link was attached.
    Attached,
    /// Its content was there already.
    AlreadyThere,
    /// It was kept with a mark of danger.
    Marked(Danger),
    /// A file was written here.
    Extracted(PathBuf),
    /// An attachment was removed.
    Removed,
    /// The components of an attachment changed.
    Changed,
    /// A settings file with a credential was refused.
    HoldsCredential(String),
    /// Another program saved the project with this name since it was read: nothing was written.
    ChangedOnDisk(String),
    /// It could not be done, in the words of what refused it.
    Failed(String),
}

impl Message {
    /// The words of the message.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Message::Attached => tr(lang, Text::Attached).to_owned(),
            Message::AlreadyThere => tr(lang, Text::AlreadyThere).to_owned(),
            Message::Marked(danger) => fill(
                tr(lang, Text::Marked),
                &[attachments::danger_word(*danger).tr(lang.code())],
            ),
            Message::Extracted(path) => {
                fill(tr(lang, Text::Extracted), &[&path.display().to_string()])
            }
            Message::Removed => tr(lang, Text::Removed).to_owned(),
            Message::Changed => tr(lang, Text::Changed).to_owned(),
            Message::HoldsCredential(name) => fill(tr(lang, Text::HoldsCredential), &[name]),
            Message::ChangedOnDisk(name) => fill(tr(lang, Text::ChangedOnDisk), &[name]),
            Message::Failed(why) => fill(tr(lang, Text::Failed), &[why]),
        }
    }

    /// Whether it asks for attention.
    pub fn is_warning(&self) -> bool {
        matches!(
            self,
            Message::Marked(_)
                | Message::HoldsCredential(_)
                | Message::ChangedOnDisk(_)
                | Message::Failed(_)
        )
    }
}

/// The SHA-256 and the size of a file, or why it could not be read.
pub type Hashed = Result<(String, u64), String>;

/// What the thread that did a request answers.
struct Done {
    /// The project that was worked on.
    file: PathBuf,
    request: Request,
    outcome: Outcome,
}

enum Outcome {
    /// Done: what to say, the attachment to select and whether the project was written.
    Finished {
        message: Message,
        select: Option<String>,
        wrote: bool,
    },
    /// The content itself says the file needs care: ask first.
    NeedsCare {
        name: String,
        danger: Option<Danger>,
    },
}

fn failed(why: impl ToString) -> Outcome {
    Outcome::Finished {
        message: Message::Failed(why.to_string()),
        select: None,
        wrote: false,
    }
}

/// The identity of a version of a file on disk.
type Stamp = Option<(SystemTime, u64)>;

fn stamp_of(path: &Path) -> Stamp {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Opens a project to look at it: the content of an attachment is checked against its own SHA-256 while it is read, so the whole
/// project is not hashed.
fn open_to_read(path: &Path) -> Result<Project, String> {
    let options = OpenOptions {
        spec: eh3d(),
        verify_integrity: false,
    };
    Project::open(path, &options).map_err(|e| e.to_string())
}

/// The file that is attached, when it is a settings file that gives a value to one of the keys.
fn holds_credential(path: &Path, keys: &[String]) -> bool {
    let small = fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() <= MAX_SETTINGS_BYTES);
    small
        && fs::read(path).is_ok_and(|bytes| keys.iter().any(|key| suiteview::sets_key(&bytes, key)))
}

fn refused_as_changed(file: &Path) -> Outcome {
    Outcome::Finished {
        message: Message::ChangedOnDisk(suiteview::name_of(file)),
        select: None,
        wrote: false,
    }
}

fn written(
    file: &Path,
    done: Result<attachments::Attached, AttachError>,
    what: Written,
) -> Outcome {
    let mut done = match done {
        Ok(done) => done,
        // saved by another program while this one was writing
        Err(e) if e.to_string().contains(PkgCode::ChangedOnDisk.code()) => {
            return refused_as_changed(file)
        }
        Err(e) => return failed(e),
    };
    let message = match what {
        Written::Attach => {
            let danger = read_index(&mut done.project)
                .ok()
                .and_then(|index| index.find(&done.id).ok().and_then(Attachment::danger));
            match danger {
                Some(danger) => Message::Marked(danger),
                None if done.already_there => Message::AlreadyThere,
                None => Message::Attached,
            }
        }
        Written::Remove => Message::Removed,
        Written::Components => Message::Changed,
    };
    Outcome::Finished {
        message,
        select: (what != Written::Remove).then(|| done.id.clone()),
        wrote: done.written,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Written {
    Attach,
    Remove,
    Components,
}

/// Does one request on the project at `file`.
fn perform(
    file: &Path,
    request: &Request,
    agreed: bool,
    attacher: &Attacher,
    credential_keys: &[String],
    seen: Option<&Revision>,
) -> Outcome {
    let options = SaveOptions::default();
    if let Request::Extract(id, folder) = request {
        let consent = if agreed {
            Consent::Given
        } else {
            Consent::NotAsked
        };
        let mut project = match open_to_read(file) {
            Ok(project) => project,
            Err(e) => return failed(e),
        };
        return match attachments::extract(&mut project, id, folder, consent, &attacher.limits) {
            Ok(path) => Outcome::Finished {
                message: Message::Extracted(path),
                select: Some(id.clone()),
                wrote: false,
            },
            Err(AttachError::NeedsConfirmation { name, danger }) => {
                Outcome::NeedsCare { name, danger }
            }
            Err(e) => failed(e),
        };
    }
    if let Request::AttachFile(path) = request {
        if holds_credential(path, credential_keys) {
            return Outcome::Finished {
                message: Message::HoldsCredential(suiteview::name_of(path)),
                select: None,
                wrote: false,
            };
        }
    }
    // what the person saw in the panel is what is written to
    if seen.is_some_and(|seen| revision::check_unchanged(file, seen).is_err()) {
        return refused_as_changed(file);
    }
    let project = match appbranch::open_project(file) {
        Ok(project) => project,
        Err(e) => return failed(e),
    };
    match request {
        Request::AttachFile(path) => written(
            file,
            attacher.attach_file(project, path, &AttachRequest::new(), &options),
            Written::Attach,
        ),
        Request::AttachLink { url, sha256, size } => written(
            file,
            attacher.attach_link(project, url, sha256, *size, &AttachRequest::new(), &options),
            Written::Attach,
        ),
        Request::Remove(id) => written(
            file,
            attacher.detach(project, id, &options),
            Written::Remove,
        ),
        Request::AddComponent(id, link) => written(
            file,
            attacher.set_components(project, id, std::slice::from_ref(link), &[], &options),
            Written::Components,
        ),
        Request::RemoveComponent(id, link) => written(
            file,
            attacher.set_components(project, id, &[], std::slice::from_ref(link), &options),
            Written::Components,
        ),
        Request::Extract(..) => failed("extract"),
    }
}

/// The state of the attachment panel.
pub struct Model {
    program: Program,
    /// The settings keys whose value never goes into a project (`server_token`).
    pub credential_keys: Vec<String>,
    limits: AttachmentLimits,
    file: Option<PathBuf>,
    stamp: Stamp,
    /// The mark of the content that was read, which a write is held to.
    revision: Option<Revision>,
    index: AttachmentIndex,
    items: Vec<Item>,
    error: Option<String>,
    selected: Option<String>,
    filter: Option<Piece>,
    pending: Option<Question>,
    job: Option<Receiver<Done>>,
    message: Option<Message>,
    wrote: bool,
    /// Dropped files that wait for their turn; true when the pointer said they are for the panel.
    dropped: VecDeque<(PathBuf, bool)>,
    hash_job: Option<Receiver<Hashed>>,
    hashed: Option<Hashed>,
}

impl Model {
    /// A panel of a program, with no project.
    pub fn new(program: Program) -> Model {
        Model {
            program,
            credential_keys: vec!["server_token".to_owned()],
            limits: AttachmentLimits::default(),
            file: None,
            stamp: None,
            revision: None,
            index: AttachmentIndex::default(),
            items: Vec::new(),
            error: None,
            selected: None,
            filter: None,
            pending: None,
            job: None,
            message: None,
            wrote: false,
            dropped: VecDeque::new(),
            hash_job: None,
            hashed: None,
        }
    }

    /// Takes one line of a settings file (`attachments.max_mb`); false for another key.
    pub fn apply_setting(&mut self, key: &str, value: &str) -> bool {
        self.limits.apply(key, value)
    }

    /// Takes the limits of attaching and of taking out, in place of the ones there were.
    pub fn set_limits(&mut self, limits: AttachmentLimits) {
        self.limits = limits;
    }

    /// The limits of attaching and of taking out.
    pub fn limits(&self) -> AttachmentLimits {
        self.limits
    }

    /// Takes the SHA-256 and the size of a file on a thread of its own; [`Model::take_hash`] gives them once, after a
    /// [`Model::poll`] that found them. A file asked for before and not taken yet is forgotten.
    pub fn start_hash(&mut self, path: PathBuf) {
        self.drop_hash();
        let (send, receive) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("eh3d-attachment-hash".to_owned())
            .spawn(move || {
                let hashed = attachments::hash_file(&path).map_err(|e| e.to_string());
                // the panel may not want it any more
                let _ = send.send(hashed);
            });
        match spawned {
            Ok(_) => self.hash_job = Some(receive),
            Err(e) => self.hashed = Some(Err(e.to_string())),
        }
    }

    /// True while the SHA-256 of a file is being taken.
    pub fn hashing(&self) -> bool {
        self.hash_job.is_some()
    }

    /// The SHA-256 that was asked for, once. A failure is also said in [`Model::message`].
    pub fn take_hash(&mut self) -> Option<Hashed> {
        let hashed = self.hashed.take();
        if let Some(Err(why)) = &hashed {
            self.message = Some(Message::Failed(why.clone()));
        }
        hashed
    }

    /// Forgets the SHA-256 that was asked for.
    pub fn drop_hash(&mut self) {
        self.hash_job = None;
        self.hashed = None;
    }

    /// Takes files dropped on the window, one after another. `over` is true when the pointer was over the panel; otherwise
    /// each file is asked about before it is attached. Folders are left out, and nothing happens with no project.
    pub fn drop_files(&mut self, paths: &[PathBuf], over: bool) {
        if self.file.is_none() {
            return;
        }
        for path in paths.iter().filter(|p| p.is_file()) {
            self.dropped.push_back((path.clone(), over));
        }
        self.next_dropped();
    }

    /// The next dropped file, when nothing else is being done or asked.
    fn next_dropped(&mut self) -> bool {
        if self.job.is_some() || self.pending.is_some() {
            return false;
        }
        let Some((path, over)) = self.dropped.pop_front() else {
            return false;
        };
        let first = (!over).then(|| Ask::Dropped(suiteview::name_of(&path)));
        self.request_after(Request::AttachFile(path), first);
        true
    }

    /// True while dropped files wait for their turn.
    pub fn waiting(&self) -> bool {
        !self.dropped.is_empty()
    }

    /// The project the program works on. Another one than before is read at once; a question that was waiting is dropped.
    pub fn set_project(&mut self, project: Option<&Path>) {
        if self.file.as_deref() == project {
            return;
        }
        self.file = project.map(Path::to_path_buf);
        self.selected = None;
        self.pending = None;
        self.message = None;
        self.dropped.clear();
        self.drop_hash();
        self.reload();
    }

    /// The project.
    pub fn project(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Reads the project again when the file changed since it was read. True when it did. A question that was waiting stays when
    /// only the date changed; when the content changed it is dropped, with nothing written, and said.
    pub fn check_file(&mut self) -> bool {
        let Some(file) = &self.file else {
            return false;
        };
        if self.job.is_some() || stamp_of(file) == self.stamp {
            return false;
        }
        let seen = self.revision.clone();
        self.reload();
        let same_content = seen.is_some() && seen == self.revision;
        if !same_content && self.pending.take().is_some() {
            let name = self.file.as_deref().map(suiteview::name_of);
            self.message = name.map(Message::ChangedOnDisk);
        }
        true
    }

    /// Reads the list of attachments again.
    pub fn reload(&mut self) {
        self.index = AttachmentIndex::default();
        self.items.clear();
        self.error = None;
        self.stamp = None;
        self.revision = None;
        let Some(file) = self.file.clone() else {
            return;
        };
        self.stamp = stamp_of(&file);
        // a project that is not written yet has no attachments
        if self.stamp.is_none() {
            return;
        }
        let read = open_to_read(&file).and_then(|mut project| {
            let index = read_index(&mut project).map_err(|e| e.to_string())?;
            Ok((index, project.revision()))
        });
        match read {
            Ok((index, revision)) => {
                self.revision = Some(revision);
                self.items = index.attachments.iter().map(Item::of).collect();
                self.items
                    .sort_by_key(|i| (i.name.to_lowercase(), i.id.clone()));
                self.index = index;
            }
            Err(e) => self.error = Some(e),
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|id| !self.items.iter().any(|i| &i.id == id))
        {
            self.selected = None;
        }
    }

    /// Why the project could not be read.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Every attachment.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The attachments to list: all of them, or the ones of the piece of the filter, the most specific first.
    pub fn visible(&self) -> Vec<&Item> {
        match &self.filter {
            None => self.items.iter().collect(),
            Some(piece) => self
                .index
                .for_component(&piece.query())
                .into_iter()
                .filter_map(|record| self.item(record.id()))
                .collect(),
        }
    }

    /// Shows only the attachments of a piece of the design (a program calls this from `R5` or `U3`), or all of them.
    pub fn set_filter(&mut self, piece: Option<Piece>) {
        self.filter = piece;
    }

    /// The piece whose attachments are listed.
    pub fn filter(&self) -> Option<&Piece> {
        self.filter.as_ref()
    }

    /// One attachment.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Chooses an attachment of the list.
    pub fn select(&mut self, id: Option<&str>) {
        self.selected = id.filter(|id| self.item(id).is_some()).map(str::to_owned);
    }

    /// The attachment chosen.
    pub fn selected(&self) -> Option<&Item> {
        self.selected.as_deref().and_then(|id| self.item(id))
    }

    /// The bytes of an attachment that is inside the project, for the viewer.
    pub fn source(&self, id: &str) -> Option<Result<AttachmentSource, String>> {
        let file = self.file.as_deref()?;
        let item = self.item(id)?;
        (!item.is_link).then(|| AttachmentSource::open(file, id).map_err(|e| e.to_string()))
    }

    /// The question that waits for the person.
    pub fn pending(&self) -> Option<&Question> {
        self.pending.as_ref()
    }

    /// What there is to say about the last thing done.
    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    /// True while a request is being done.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// True once after the project was written: the program reads its own state of the project again.
    pub fn take_wrote(&mut self) -> bool {
        std::mem::take(&mut self.wrote)
    }

    /// What has to be agreed to before a request is done.
    fn questions(&self, file: &Path, request: &Request) -> Result<Vec<Ask>, String> {
        let mut lines = Vec::new();
        match request {
            Request::Extract(id, _) => {
                if let Some(item) = self.item(id).filter(|i| i.needs_confirmation && !i.is_link) {
                    lines.push(Ask::Care {
                        name: item.name.clone(),
                        danger: item.danger,
                    });
                }
                return Ok(lines);
            }
            Request::Remove(id) => {
                if let Some(item) = self.item(id) {
                    lines.push(Ask::Remove {
                        name: item.name.clone(),
                        components: item.components.len(),
                    });
                }
            }
            _ => {}
        }
        let view = suiteview::view(file, &self.program.branch)?;
        if view.project_signed {
            lines.push(Ask::SignedProject);
        }
        if view.branch(ATTACHMENTS_BRANCH).is_some_and(|b| b.signed) {
            lines.push(Ask::SignedAttachments);
        }
        Ok(lines)
    }

    /// Asks for something. It is done at once on a thread of its own, or waits as [`Model::pending`] for the person to agree.
    /// Nothing happens with no project or while another request is being done.
    pub fn request(&mut self, request: Request) {
        self.request_after(request, None);
    }

    /// A request with a line that is asked before the ones of the request itself.
    fn request_after(&mut self, request: Request, first: Option<Ask>) {
        let Some(file) = self.file.clone() else {
            return;
        };
        if self.job.is_some() {
            return;
        }
        self.pending = None;
        self.message = None;
        let questions = self.questions(&file, &request).map(|mut lines| {
            if let Some(first) = first {
                lines.insert(0, first);
            }
            lines
        });
        match questions {
            Ok(lines) if lines.is_empty() => self.start(file, request, false),
            Ok(lines) => self.pending = Some(Question { request, lines }),
            Err(e) => self.message = Some(Message::Failed(e)),
        }
    }

    /// The person agreed to the question.
    pub fn confirm(&mut self) {
        let (Some(file), None) = (self.file.clone(), &self.job) else {
            return;
        };
        if let Some(question) = self.pending.take() {
            self.start(file, question.request, true);
        }
    }

    /// The person did not agree: nothing is done.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    fn start(&mut self, file: PathBuf, request: Request, agreed: bool) {
        let attacher =
            Attacher::new(&self.program.branch, &self.program.version).with_limits(self.limits);
        let keys = self.credential_keys.clone();
        let seen = self.revision.clone();
        let (send, receive) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("eh3d-attachments".to_owned())
            .spawn(move || {
                let outcome = perform(&file, &request, agreed, &attacher, &keys, seen.as_ref());
                // the panel may be gone: there is nobody to tell then
                let _ = send.send(Done {
                    file,
                    request,
                    outcome,
                });
            });
        match spawned {
            Ok(_) => self.job = Some(receive),
            Err(e) => self.message = Some(Message::Failed(e.to_string())),
        }
    }

    /// Collects the answers of the threads and goes on with the next dropped file. Call it on every frame; true when
    /// something changed.
    pub fn poll(&mut self) -> bool {
        let hashed = self.poll_hash();
        let done = self.poll_job();
        let next = self.next_dropped();
        hashed || done || next
    }

    fn poll_hash(&mut self) -> bool {
        let Some(job) = &self.hash_job else {
            return false;
        };
        self.hashed = match job.try_recv() {
            Ok(hashed) => Some(hashed),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Some(Err("the work stopped".to_owned())),
        };
        self.hash_job = None;
        true
    }

    fn poll_job(&mut self) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        let done = match job.try_recv() {
            Ok(done) => done,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.message = Some(Message::Failed("the work stopped".to_owned()));
                self.reload();
                return true;
            }
        };
        self.job = None;
        // another project was opened meanwhile: the answer is not about what is shown
        let same = self.file.as_deref() == Some(done.file.as_path());
        match done.outcome {
            Outcome::Finished {
                message,
                select,
                wrote,
            } => {
                self.wrote |= wrote;
                // a refusal is not covered by the next file: the dropped files that wait are given up
                if matches!(
                    message,
                    Message::Failed(_) | Message::HoldsCredential(_) | Message::ChangedOnDisk(_)
                ) {
                    self.dropped.clear();
                }
                if same {
                    self.message = Some(message);
                    self.reload();
                    if let Some(id) = select {
                        self.select(Some(&id));
                    }
                }
            }
            Outcome::NeedsCare { name, danger } => {
                if same {
                    self.pending = Some(Question {
                        request: done.request,
                        lines: vec![Ask::Care { name, danger }],
                    });
                }
            }
        }
        true
    }
}
