//! `eh3d-project`: look into a `.eh3d` project from a terminal.
//!
//! ```text
//! eh3d-project info FILE [--json]
//! eh3d-project verify FILE [--strict] [--require-signed BRANCH[,BRANCH...]]
//! eh3d-project extract-branch FILE BRANCH OUT_DIR
//! eh3d-project extract-branch FILE manufacturing --eh3dproj OUT.eh3dproj
//! ```
//!
//! Exit codes: 0 sound, 2 verification not passed (an error finding, or a warning with
//! `--strict`), 1 anything else that failed, 64 wrong use. Nothing is written except by
//! `extract-branch`, which never replaces a file.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use eh3d_container::AtomicOptions;
use eh3d_project::{Checks, OpenOptions, Project};

const USAGE: &str = "usage:\n  eh3d-project info FILE [--json]\n  eh3d-project verify FILE [--strict] [--require-signed BRANCH[,BRANCH...]]\n  eh3d-project extract-branch FILE BRANCH OUT_DIR\n  eh3d-project extract-branch FILE manufacturing --eh3dproj OUT.eh3dproj";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let outcome = match args.first().map(String::as_str) {
        Some("info") => info(&args[1..]),
        Some("verify") => verify(&args[1..]),
        Some("extract-branch") => extract(&args[1..]),
        _ => Err(Failure::Usage),
    };
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(Failure::Usage) => {
            eprintln!("{USAGE}");
            ExitCode::from(64)
        }
        Err(Failure::Error(text)) => {
            eprintln!("error: {text}");
            ExitCode::from(1)
        }
    }
}

enum Failure {
    Usage,
    Error(String),
}

impl From<eh3d_container::ContainerError> for Failure {
    fn from(e: eh3d_container::ContainerError) -> Self {
        Failure::Error(e.to_string())
    }
}

fn open(path: &str) -> Result<Project, Failure> {
    Ok(Project::open(Path::new(path), &OpenOptions::default())?)
}

fn info(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let json = match rest {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => return Err(Failure::Usage),
    };
    let project = open(file)?;
    let record = project.record();
    let stale = project.stale_refs();
    if json {
        let branches: Vec<serde_json::Value> = project
            .branches()
            .map(|b| {
                serde_json::json!({
                    "id": b.id,
                    "known": b.is_known(),
                    "files": b.files.len(),
                    "hash": b.hash,
                    "app": b.record.as_ref().map(|r| r.app.clone()),
                    "format": b.record.as_ref().map(|r| r.format.clone()),
                    "app_version": b.record.as_ref().map(|r| r.app_version.clone()),
                    "sealed": b.is_sealed(),
                    "encrypted": b.is_encrypted(),
                    "signed": b.record.as_ref().is_some_and(|r| r.signature.is_some()),
                    "out_of_date": stale.iter().any(|s| s.branch == b.id),
                })
            })
            .collect();
        let document = serde_json::json!({
            "id": record.id,
            "name": record.name,
            "format_version": record.format_version,
            "container_version": record.container_version,
            "units": record.units,
            "sealed": record.seal.is_some(),
            "content_hash": project.content_hash()?,
            "branches": branches,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&document).unwrap_or_default()
        );
        return Ok(0);
    }
    println!("project   {} ({})", record.name, record.id);
    println!(
        "format    {} (container rules {})",
        record.format_version, record.container_version
    );
    println!("units     {}", record.units);
    println!(
        "sealed    {}",
        if record.seal.is_some() { "yes" } else { "no" }
    );
    println!("content   {}", project.content_hash()?);
    println!("branches  {}", project.branches().count());
    for b in project.branches() {
        let (app, format, version) = b.record.as_ref().map_or(("-", "-", "-"), |r| {
            (r.app.as_str(), r.format.as_str(), r.app_version.as_str())
        });
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
        if stale.iter().any(|s| s.branch == b.id) {
            notes.push("OUT OF DATE");
        }
        println!(
            "  {:<16} {:>4} files  format {:<6} {} {}  {}",
            b.id,
            b.files.len(),
            format,
            app,
            version,
            notes.join(", ")
        );
    }
    for s in &stale {
        println!(
            "  '{}' was derived from '{}' ({}) before it changed",
            s.branch, s.dependency.branch, s.dependency.artifact
        );
    }
    Ok(0)
}

fn verify(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let mut strict = false;
    let mut required: BTreeSet<String> = BTreeSet::new();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--strict" => strict = true,
            "--require-signed" => {
                i += 1;
                let list = rest.get(i).ok_or(Failure::Usage)?;
                required.extend(list.split(',').filter(|s| !s.is_empty()).map(str::to_owned));
            }
            _ => return Err(Failure::Usage),
        }
        i += 1;
    }
    let checks = Checks {
        require_signed: required,
        ..Checks::default()
    };
    let mut project = Project::open(Path::new(file), &OpenOptions::default())?;
    let findings = project.diagnose(&checks);
    let mut failed = false;
    for f in &findings {
        let level = if f.is_error() { "error" } else { "warning" };
        println!(
            "{level} {}: {} [{}]{}",
            f.code,
            f.detail,
            f.branch.as_deref().unwrap_or("project"),
            f.entry
                .as_deref()
                .map_or(String::new(), |e| format!(" {e}"))
        );
        failed |= f.is_error() || strict;
    }
    if failed {
        println!("VERIFY=FAIL");
        Ok(2)
    } else {
        println!("VERIFY=PASS");
        Ok(0)
    }
}

fn extract(args: &[String]) -> Result<u8, Failure> {
    match args {
        [file, branch, out] => {
            let mut project = open(file)?;
            let count = project.extract_branch(branch, Path::new(out))?;
            println!("{count} files written to {out}");
            Ok(0)
        }
        [file, branch, flag, out] if flag == "--eh3dproj" && branch == "manufacturing" => {
            let mut project = open(file)?;
            project.export_manufacturing(Path::new(out), &AtomicOptions::default())?;
            println!("package written to {out}");
            Ok(0)
        }
        _ => Err(Failure::Usage),
    }
}
