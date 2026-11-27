//! The `.eh3d` project as the manufacturing programs use it (viewer, stencils, isolation, drilling, routing). The branch `manufacturing/` holds the job as a `.eh3dproj` 1.0.0 package.
//! A program reads that job, makes its result and keeps it **in its own branch only**, with a copy of every file the result was made from under `taken/` and a reference that remembers
//! the content of the branch they came from, so a later change of the job is told as **out of date**. No other branch changes and nothing in a project is run. Each program gives its
//! own [`BranchKit`] (see [`kit`]).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

use crate::appbranch::{self, BranchError, BranchKit, Identity, TAKEN_DIR};
use crate::revision::{self, Revision};
use crate::suiteview;

pub use crate::suiteview::is_project;

/// The branch the job is read from.
pub const SOURCE_BRANCH: &str = "manufacturing";
/// The folder of a branch that holds what the program made.
pub const RESULT_DIR: &str = "result/";
/// The format of the branches of these programs, `major.minor`.
pub const FORMAT: &str = "1.0";
/// The most files of a job that are read.
pub const MAX_JOB_FILES: usize = 256;
/// The most bytes of a job that are read.
pub const MAX_JOB_BYTES: usize = 256 * 1024 * 1024;

/// The branch of a manufacturing program: its name, the file that records it and the version of the program.
pub const fn kit<'a>(branch: &'a str, manifest: &'a str, app_version: &'a str) -> BranchKit<'a> {
    BranchKit::new(branch, FORMAT, manifest, app_version)
}

/// One layer file of the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobFile {
    /// The name inside the branch `manufacturing/`, for example `layers/top.gtl`.
    pub rel: String,
    /// What the job says it is: `copper`, `solder_mask`, `silkscreen`, `paste`, `outline`, `drill` or `other`.
    pub kind: String,
    /// The name the job gives it.
    pub name: String,
    /// The file.
    pub bytes: Vec<u8>,
}

impl JobFile {
    /// The name of the file without its folders.
    pub fn file_name(&self) -> &str {
        self.rel.rsplit('/').next().unwrap_or(&self.rel)
    }
}

/// What the branch of a program holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The files of the job the result was made from, as names inside `manufacturing/`.
    pub inputs: Vec<String>,
    /// The results, as names inside `result/`.
    pub results: Vec<String>,
    /// Whether the job changed since the result was made.
    pub out_of_date: bool,
    /// What is out of date or wrong in the project.
    pub warnings: Vec<String>,
}

/// The layer files of the job of a project, only those of the `kinds` asked for (every one when none is asked for), in the order of the job.
pub fn read_job(path: &Path, kinds: &[&str]) -> Result<Vec<JobFile>, String> {
    let mut project = appbranch::open_project(path).map_err(|e| e.to_string())?;
    let file = path.display();
    if project.branch(SOURCE_BRANCH).is_none() {
        return Err(format!(
            "{file}: the project has no branch {SOURCE_BRANCH} (the job of a .eh3dproj)"
        ));
    }
    let record = project
        .read_entry(SOURCE_BRANCH, "project.json")
        .map_err(|e| format!("{file}: {e}"))?;
    let record: Value = serde_json::from_slice(&record)
        .map_err(|e| format!("{file}: {SOURCE_BRANCH}/project.json is not a valid record: {e}"))?;
    let layers = record
        .get("layers")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{file}: {SOURCE_BRANCH}/project.json has no list of layers"))?;
    if layers.len() > MAX_JOB_FILES {
        return Err(format!(
            "{file}: the job has {} layers, more than {MAX_JOB_FILES}",
            layers.len()
        ));
    }
    let mut out: Vec<JobFile> = Vec::new();
    let mut total = 0usize;
    for layer in layers {
        let text = |key: &str| layer.get(key).and_then(Value::as_str);
        let rel = text("path")
            .filter(|p| p.starts_with("layers/"))
            .ok_or_else(|| format!("{file}: a layer of the job has no path under layers/"))?;
        let kind = text("kind").unwrap_or("other");
        if (!kinds.is_empty() && !kinds.contains(&kind)) || out.iter().any(|f| f.rel == rel) {
            continue;
        }
        let bytes = project
            .read_entry(SOURCE_BRANCH, rel)
            .map_err(|e| format!("{file}: {rel}: {e}"))?;
        total = total.saturating_add(bytes.len());
        if total > MAX_JOB_BYTES {
            return Err(format!(
                "{file}: the layers of the job are above {MAX_JOB_BYTES} bytes"
            ));
        }
        let name = text("name")
            .unwrap_or_else(|| rel.rsplit('/').next().unwrap_or(rel))
            .to_string();
        out.push(JobFile {
            rel: rel.to_string(),
            kind: kind.to_string(),
            name,
            bytes,
        });
    }
    Ok(out)
}

/// The layers of one kind of the job; an error when the job has none.
pub fn layers_of(path: &Path, kind: &str) -> Result<Vec<JobFile>, String> {
    let layers = read_job(path, &[kind])?;
    if layers.is_empty() {
        return Err(format!("{}: the job has no {kind} layer", path.display()));
    }
    Ok(layers)
}

/// The layer called `layer` (its path inside the job or its file name) among those of a kind.
pub fn choose<'a>(layers: &'a [JobFile], kind: &str, layer: &str) -> Result<&'a JobFile, String> {
    layers
        .iter()
        .find(|f| f.rel == layer || f.file_name() == layer)
        .ok_or_else(|| {
            let names: Vec<&str> = layers.iter().map(|f| f.rel.as_str()).collect();
            format!(
                "the job has no {kind} layer {layer}; it has {}",
                names.join(", ")
            )
        })
}

/// The name inside `result/` of what is made from a layer: `<prefix>-<name of the layer>.<extension>`.
pub fn result_name(prefix: &str, layer: &JobFile, extension: &str) -> String {
    let file = layer.file_name();
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    format!("{prefix}-{stem}.{extension}")
}

/// Keeps the results of a program in its branch of the project at `path`, with a copy of the files of the job they were made from and a reference to the state of the job. The results and
/// copies that were there are replaced; the rest of the branch and every other branch stay. Refused when a file of the job is no longer the one that was read. Returns what a person
/// should be told.
pub fn save_result(
    kit: &BranchKit<'_>,
    path: &Path,
    inputs: &[JobFile],
    results: &[(String, Vec<u8>)],
) -> Result<Vec<String>, String> {
    write_result(kit, path, None, inputs, results)
}

/// Like [`save_result`] for a project that a window opened with the mark `seen` ([`suiteview::ProjectView::own_hash`]): refused
/// (`EH3D-PKG-025`, see [`suiteview::is_changed_refusal`]), with nothing written, when another program saved the file since, when it is
/// gone or when `seen` is not a mark of an opening. Writes of this program do not count.
pub fn save_result_since(
    kit: &BranchKit<'_>,
    path: &Path,
    seen: &str,
    inputs: &[JobFile],
    results: &[(String, Vec<u8>)],
) -> Result<Vec<String>, String> {
    let seen = opened(path, seen)?;
    write_result(kit, path, Some(&seen), inputs, results)
}

/// The revision of a mark of an opening, with the file still as it says.
fn opened(path: &Path, seen: &str) -> Result<Revision, String> {
    // a text that holds no revision cannot show that the file is the one that was read
    let seen = suiteview::revision_of(seen).ok_or_else(|| revision::changed().to_string())?;
    revision::check_unchanged(path, &seen).map_err(|e| e.to_string())?;
    Ok(seen)
}

fn write_result(
    kit: &BranchKit<'_>,
    path: &Path,
    seen: Option<&Revision>,
    inputs: &[JobFile],
    results: &[(String, Vec<u8>)],
) -> Result<Vec<String>, String> {
    if inputs.is_empty() || results.is_empty() {
        return Err("there is nothing to save: a result and the layers it was made from".into());
    }
    if !path.is_file() {
        return Err(format!("{}: there is no such project", path.display()));
    }
    // every name is checked before anything is written
    for (name, _) in results {
        kit.check_name(&format!("{RESULT_DIR}{name}"))
            .map_err(|e| e.to_string())?;
    }
    let apply = |project: &mut crate::Project,
                 copy: &mut appbranch::WorkingCopy|
     -> appbranch::BranchResult<()> {
        let old: Vec<String> = copy
            .files()
            .keys()
            .filter(|n| n.starts_with(TAKEN_DIR) || n.starts_with(RESULT_DIR))
            .cloned()
            .collect();
        for name in &old {
            copy.remove(name);
        }
        let mut targets = Vec::new();
        for input in inputs {
            let now = project.read_entry(SOURCE_BRANCH, &input.rel)?;
            if now != input.bytes {
                return Err(BranchError::Refused(format!(
                    "{} changed while the result was being made; make it again",
                    input.rel
                )));
            }
            let target = format!("{TAKEN_DIR}{SOURCE_BRANCH}/{}", input.rel);
            copy.put(&target, now)?;
            targets.push(target);
        }
        let dependency = project.dependency_on(SOURCE_BRANCH, "job-layers", 1, targets)?;
        copy.add_dependency(dependency);
        for (name, bytes) in results {
            copy.put(&format!("{RESULT_DIR}{name}"), bytes.clone())?;
        }
        copy.set("source", SOURCE_BRANCH);
        Ok(())
    };
    let written = match seen {
        Some(seen) => kit.update_since(path, seen, apply),
        None => kit.update(path, &Identity::default(), apply),
    };
    let (_, notices) = written.map_err(|e| e.to_string())?;
    Ok(notices.iter().map(ToString::to_string).collect())
}

/// Keeps one result made from one layer of the job. The results of the other layers that are already there stay when they are up to date; when the job changed since they were made they
/// go, because what they were made from is no longer the job. Returns what a person should be told.
pub fn keep_result(
    kit: &BranchKit<'_>,
    path: &Path,
    layer: &JobFile,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Vec<String>, String> {
    keep(kit, path, None, layer, name, bytes)
}

/// Like [`keep_result`] for a project that a window opened with the mark `seen`, refused as [`save_result_since`] is. The results of the
/// other layers that are kept are therefore the ones the person saw, never ones that another program wrote meanwhile.
pub fn keep_result_since(
    kit: &BranchKit<'_>,
    path: &Path,
    seen: &str,
    layer: &JobFile,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Vec<String>, String> {
    let seen = opened(path, seen)?;
    keep(kit, path, Some(&seen), layer, name, bytes)
}

fn keep(
    kit: &BranchKit<'_>,
    path: &Path,
    seen: Option<&Revision>,
    layer: &JobFile,
    name: &str,
    bytes: Vec<u8>,
) -> Result<Vec<String>, String> {
    let before = state(kit, path)?;
    let mut inputs = vec![layer.clone()];
    let mut results = vec![(name.to_string(), bytes)];
    if !before.out_of_date {
        let job = read_job(path, &[])?;
        inputs.extend(
            job.into_iter()
                .filter(|f| f.rel != layer.rel && before.inputs.contains(&f.rel)),
        );
        for other in before.results.iter().filter(|r| r.as_str() != name) {
            results.push((other.clone(), read_result(kit, path, other)?));
        }
    }
    write_result(kit, path, seen, &inputs, &results)
}

/// What the branch of a program holds in the project at `path`, and whether it is out of date. A project without the branch has an empty state.
pub fn state(kit: &BranchKit<'_>, path: &Path) -> Result<State, String> {
    let mut project = appbranch::open_project(path).map_err(|e| e.to_string())?;
    let copy = kit.load(&mut project).map_err(|e| e.to_string())?;
    let taken = format!("{TAKEN_DIR}{SOURCE_BRANCH}/");
    let under = |prefix: &str| -> Vec<String> {
        copy.files()
            .keys()
            .filter_map(|n| n.strip_prefix(prefix))
            .map(str::to_string)
            .collect()
    };
    Ok(State {
        inputs: under(&taken),
        results: under(RESULT_DIR),
        out_of_date: project.stale_refs().iter().any(|s| s.branch == kit.branch),
        warnings: appbranch::warnings(&mut project),
    })
}

/// One result of the branch of a program.
pub fn read_result(kit: &BranchKit<'_>, path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let mut project = appbranch::open_project(path).map_err(|e| e.to_string())?;
    let copy = kit.load(&mut project).map_err(|e| e.to_string())?;
    copy.file(&format!("{RESULT_DIR}{name}"))
        .map(<[u8]>::to_vec)
        .ok_or_else(|| {
            format!(
                "{}: the branch {} has no result {name}",
                path.display(),
                kit.branch
            )
        })
}

/// The lines that say what the project holds for a program: the job, the result and what is out of date.
pub fn describe(kit: &BranchKit<'_>, path: &Path) -> Result<(Vec<String>, State), String> {
    let job = read_job(path, &[])?;
    let state = state(kit, path)?;
    let mut lines = vec![format!("{SOURCE_BRANCH}: {} layers", job.len())];
    lines.extend(job.iter().map(|f| format!("  {} ({})", f.rel, f.kind)));
    if state.results.is_empty() {
        lines.push(format!("{}: no result yet", kit.branch));
    } else {
        lines.push(format!(
            "{}: {} made from {}",
            kit.branch,
            state.results.join(", "),
            state.inputs.join(", ")
        ));
    }
    lines.extend(state.warnings.iter().cloned());
    Ok((lines, state))
}

/// The message of a result that is out of date; it starts as a verification that did not pass.
pub fn out_of_date_message(kit: &BranchKit<'_>) -> String {
    format!(
        "verification failed: the result of {} is OUT OF DATE: {SOURCE_BRANCH} changed since it was made",
        kit.branch
    )
}

/// A new empty folder of the system for a program to work in; the caller removes it.
pub fn scratch_dir(program: &str) -> Result<PathBuf, String> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "eh3d-{program}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}
