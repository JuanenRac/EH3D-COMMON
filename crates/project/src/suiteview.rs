//! What the window of a program needs to know about a project file of the suite (`.eh3d`) before and after it writes there: its branches and their
//! state, what is out of date, what is signed, what a person has to agree to before the program writes, and a copy under another name
//! that keeps every branch. Nothing here interprets the files of a branch and only [`copy_project`] writes. The words are
//! those of [`crate::suitetext`].

use std::io::Write as _;
use std::path::Path;

use eh3d_container::{write_atomic, AtomicOptions, ContainerError};

use crate::appbranch::{self, Notice};
use crate::meta::{ConversionReport, Counted};
use crate::spec::ATTACHMENTS_BRANCH;
use crate::suitetext::{fill, Said, S};

/// The record the container keeps in every branch.
const BRANCH_RECORD: &str = "branch.json";
/// A project larger than this is not copied under another name.
const MAX_COPIED_BYTES: u64 = 2 << 30;

/// Whether a path names a project file of the suite.
pub fn is_project(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(appbranch::EXTENSION))
}

/// The path with the extension of a project when it has none.
pub fn with_extension(path: &Path) -> std::path::PathBuf {
    if path.extension().is_none() {
        path.with_extension(appbranch::EXTENSION)
    } else {
        path.to_path_buf()
    }
}

/// The name of the file of a path, for a message.
pub fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// One branch as the window lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchState {
    /// The name of the branch.
    pub id: String,
    /// The program that wrote it (`-` without a record).
    pub app: String,
    /// The format of the branch.
    pub format: String,
    /// How many files it has.
    pub files: usize,
    /// The branches it was derived from that have changed since.
    pub stale: Vec<String>,
    /// Whether its record carries a signature (which this program does not check).
    pub signed: bool,
    /// Whether it is sealed against changes.
    pub sealed: bool,
    /// Whether it is encrypted.
    pub encrypted: bool,
}

/// A derived branch whose source has changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stale {
    /// The derived branch.
    pub branch: String,
    /// The branch it was derived from.
    pub source: String,
    /// Its files that came from the source.
    pub uses: String,
}

/// What a project holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectView {
    /// The branches.
    pub branches: Vec<BranchState>,
    /// What is out of date.
    pub stale: Vec<Stale>,
    /// Whether the project as a whole is signed.
    pub project_signed: bool,
    /// The content hash of the branch of this program, when it is there.
    pub own_hash: Option<String>,
    /// The files of the other branches, as branch and name.
    pub takeable: Vec<(String, String)>,
}

impl ProjectView {
    /// The branch of a name.
    pub fn branch(&self, id: &str) -> Option<&BranchState> {
        self.branches.iter().find(|b| b.id == id)
    }
}

/// Reads what the project at `path` holds, as the program that owns the branch `own` sees it.
pub fn view(path: &Path, own: &str) -> Result<ProjectView, String> {
    let project = appbranch::open_project(path).map_err(|e| e.to_string())?;
    let stale: Vec<Stale> = project
        .stale_refs()
        .iter()
        .map(|s| Stale {
            branch: s.branch.clone(),
            source: s.dependency.branch.clone(),
            uses: s.dependency.uses.join(", "),
        })
        .collect();
    let mut found = ProjectView {
        project_signed: project.record().signature.is_some(),
        ..ProjectView::default()
    };
    for b in project.branches() {
        let record = b.record.as_ref();
        if b.id == own {
            found.own_hash = Some(b.hash.clone());
        } else if b.id != ATTACHMENTS_BRANCH {
            // the attachments are stored under their hash: they are listed and taken out as attachments, never taken as files
            for name in b.files.keys().filter(|n| n.as_str() != BRANCH_RECORD) {
                found.takeable.push((b.id.clone(), name.clone()));
            }
        }
        found.branches.push(BranchState {
            id: b.id.clone(),
            app: record.map_or_else(|| "-".to_owned(), |r| r.app.clone()),
            format: record.map_or_else(|| "-".to_owned(), |r| r.format.clone()),
            files: b.files.len(),
            stale: stale
                .iter()
                .filter(|s| s.branch == b.id)
                .map(|s| s.source.clone())
                .collect(),
            signed: record.is_some_and(|r| r.signature.is_some()),
            sealed: b.is_sealed(),
            encrypted: b.is_encrypted(),
        });
    }
    found.stale = stale;
    Ok(found)
}

/// The state of a branch in words: current, or out of date with the branches that changed, and then whether it is signed, sealed or
/// encrypted.
pub fn state_text(line: &BranchState, code: &str) -> String {
    let mut words = vec![if line.stale.is_empty() {
        S::StateCurrent.tr(code).to_owned()
    } else {
        fill(S::StateStale.tr(code), &[&line.stale.join(", ")])
    }];
    if line.signed {
        words.push(S::StateSigned.tr(code).to_owned());
    }
    if line.sealed {
        words.push(S::StateSealed.tr(code).to_owned());
    }
    if line.encrypted {
        words.push(S::StateEncrypted.tr(code).to_owned());
    }
    words.join("; ")
}

/// What is always in sight about a project: its name, what is out of date and what a write would unsign. The flag says that a line
/// asks for attention.
pub fn banner(
    file: &Path,
    view: Option<&ProjectView>,
    own: &str,
    code: &str,
) -> Vec<(String, bool)> {
    let mut lines = vec![(fill(S::Project.tr(code), &[&name_of(file)]), false)];
    let Some(view) = view else {
        return lines;
    };
    for s in &view.stale {
        let said = fill(S::OutOfDate.tr(code), &[&s.branch, &s.source, &s.uses]);
        lines.push((said, true));
    }
    if view.project_signed {
        lines.push((S::SignedProject.tr(code).to_owned(), true));
    }
    if view.branch(own).is_some_and(|b| b.signed) {
        lines.push((fill(S::SignedBranch.tr(code), &[own]), true));
    }
    lines
}

/// What a person has to agree to before the branch of the program is written into `target`. `current` is the project the window works
/// on (also the source of a copy when `target` is not there yet) and `seen` the content hash its branch had when the window read or
/// wrote it. Nothing to agree to gives an empty list.
pub fn warnings(
    target: &Path,
    current: Option<&Path>,
    seen: Option<&str>,
    own: &str,
) -> Result<Vec<Said>, String> {
    let mut out = Vec::new();
    let found = if target.exists() {
        let found = view(target, own)?;
        if !current.is_some_and(|c| same_file(c, target)) {
            out.push(Said(S::WarnForeign, vec![name_of(target), own.to_owned()]));
        } else if found.own_hash.as_deref() != seen {
            out.push(Said(S::WarnChanged, vec![own.to_owned(), name_of(target)]));
        }
        Some(found)
    } else {
        // a new file gets the branches of the project that is open, signatures included
        match current.filter(|c| c.exists()) {
            Some(c) => Some(view(c, own)?),
            None => None,
        }
    };
    if let Some(found) = found {
        if found.project_signed {
            out.push(Said::plain(S::SignedProject));
        }
        if found.branch(own).is_some_and(|b| b.signed) {
            out.push(Said(S::SignedBranch, vec![own.to_owned()]));
        }
    }
    Ok(out)
}

/// Whether two paths name the same file.
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Copies a project under another name with every branch as it is, in one step. The source is checked first and a file that is there
/// is never written over.
pub fn copy_project(from: &Path, to: &Path) -> Result<(), String> {
    if to.exists() {
        return Err(format!("{}: the file is there already", to.display()));
    }
    drop(appbranch::open_project(from).map_err(|e| e.to_string())?);
    let bytes = appbranch::read_bounded(from, MAX_COPIED_BYTES).map_err(|e| e.to_string())?;
    write_atomic(to, &AtomicOptions::default(), |file| {
        file.write_all(&bytes).map_err(ContainerError::Io)
    })
    .map_err(|e| e.to_string())
}

/// The notices of a write, as the shared module says them, in the words of the window. One it does not know is not lost: it stays as
/// it was said.
pub fn notices(said: &[String]) -> (Vec<Said>, Vec<String>) {
    let known = [
        (Notice::ProjectSignatureRemoved, S::NoticeProjectSignature),
        (Notice::BranchSignatureRemoved, S::NoticeBranchSignature),
        (Notice::ImportReplaced, S::NoticeImportReplaced),
    ];
    let mut words = Vec::new();
    let mut other = Vec::new();
    for line in said {
        match known.iter().find(|(n, _)| n.to_string() == *line) {
            Some((_, word)) => words.push(Said::plain(*word)),
            None => other.push(line.clone()),
        }
    }
    (words, other)
}

/// The lines that tell what the report of an original says.
pub fn report_lines(report: &ConversionReport) -> Vec<Said> {
    let list = |items: &[Counted]| {
        items
            .iter()
            .map(|c| match &c.reason {
                Some(reason) => format!("{} {} ({reason})", c.item, c.count),
                None => format!("{} {}", c.item, c.count),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = vec![Said(
        S::Original,
        vec![report.source.name.clone(), report.source.size.to_string()],
    )];
    for (word, items) in [
        (S::ReportRead, &report.read),
        (S::ReportNotRead, &report.not_read),
        (S::ReportLost, &report.export_losses),
    ] {
        if !items.is_empty() {
            out.push(Said(word, vec![list(items)]));
        }
    }
    out
}

/// Whether a settings file of `key = value` lines gives a value to `key`. A project is a file that is passed on, so a program asks this before it puts one of its settings files in a
/// project and leaves out (or refuses) the one that holds a credential. It looks at every line of every section, without regard to case, and does not need the file to be well formed:
/// a file the program cannot read is still searched.
pub fn sets_key(bytes: &[u8], key: &str) -> bool {
    String::from_utf8_lossy(bytes)
        .split(['\n', '\r'])
        .any(|line| {
            let line = line.trim().trim_start_matches('\u{feff}');
            line.split_once('=').is_some_and(|(name, value)| {
                name.trim().eq_ignore_ascii_case(key) && !value.trim().is_empty()
            })
        })
}
