//! The command line `eh3d-project info|verify|extract-branch` on the synthetic project.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::*;
use tempfile::tempdir;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eh3d-project"))
        .args(args)
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn project(dir: &Path) -> String {
    synthetic(dir).path().to_string_lossy().into_owned()
}

#[test]
fn info_lists_the_branches_in_text_and_in_json() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let text = run(&["info", &file]);
    assert_eq!(text.status.code(), Some(0));
    let text = out(&text);
    assert!(text.contains("Synthetic test project"));
    assert!(text.contains("branches  8"));
    assert!(text.contains("manufacturing"));

    let json = run(&["info", &file, "--json"]);
    assert_eq!(json.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_str(&out(&json)).unwrap();
    assert_eq!(value["branches"].as_array().unwrap().len(), 8);
    assert_eq!(value["format_version"], "1.0.0");
}

#[test]
fn verify_passes_a_sound_project_and_fails_a_damaged_one() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let ok = run(&["verify", &file]);
    assert_eq!(ok.status.code(), Some(0));
    assert!(out(&ok).contains("VERIFY=PASS"));

    let damaged = dir.path().join("damaged.eh3d");
    std::fs::copy(&file, &damaged).unwrap();
    flip_in_entry(&damaged, "pcb/outline.json");
    let bad = run(&["verify", damaged.to_str().unwrap()]);
    assert_ne!(bad.status.code(), Some(0));
}

#[test]
fn verify_asks_for_signatures_when_told_to() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let unsigned = run(&["verify", &file, "--require-signed", "pcb"]);
    assert_eq!(unsigned.status.code(), Some(2));
    let text = out(&unsigned);
    assert!(text.contains("EH3D-PKG-022"), "{text}");
    assert!(text.contains("VERIFY=FAIL"));
}

#[test]
fn extract_branch_writes_the_files_and_never_replaces_one() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let target = dir.path().join("cnc-out");
    let first = run(&["extract-branch", &file, "cnc", target.to_str().unwrap()]);
    assert_eq!(first.status.code(), Some(0));
    assert!(target.join("branch.json").is_file());
    assert!(target.join("part.json").is_file());
    let second = run(&["extract-branch", &file, "cnc", target.to_str().unwrap()]);
    assert_eq!(second.status.code(), Some(1));

    let package = dir.path().join("typical.eh3dproj");
    let made = run(&[
        "extract-branch",
        &file,
        "manufacturing",
        "--eh3dproj",
        package.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", out(&made));
    assert_eq!(
        std::fs::read(&package).unwrap(),
        std::fs::read(Path::new(GOLDEN).join("typical.eh3dproj")).unwrap()
    );
}

#[test]
fn wrong_use_and_missing_files_have_their_own_exit_codes() {
    assert_eq!(run(&[]).status.code(), Some(64));
    assert_eq!(run(&["info"]).status.code(), Some(64));
    assert_eq!(run(&["frobnicate", "x"]).status.code(), Some(64));
    assert_eq!(run(&["--help"]).status.code(), Some(0));
    assert_eq!(run(&["info", "no-such-file.eh3d"]).status.code(), Some(1));
}
