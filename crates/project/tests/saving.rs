//! Saving does not compress again what did not change: the compressed bytes and the CRC of every
//! entry that stays are copied as they are, the new file is checked entry by entry before it
//! replaces the old one, and a damaged file is still found. Every file here is made up.

mod support;

use std::path::Path;

use eh3d_container::PkgCode;
use eh3d_project::spec::eh3d;
use eh3d_project::{
    AttachRequest, Attacher, BranchUpdate, Checks, OpenOptions, Project, SaveOptions, Writer,
};
use support::*;
use tempfile::tempdir;

fn open(path: &Path) -> Project {
    Project::open(path, &OpenOptions::default()).unwrap()
}

fn open_unchecked(path: &Path) -> Project {
    let options = OpenOptions {
        spec: eh3d(),
        verify_integrity: false,
    };
    Project::open(path, &options).unwrap()
}

/// Lines that repeat enough to compress and vary enough for two levels to differ.
fn made_up_program() -> Vec<u8> {
    (0..20_000)
        .map(|i| format!("G1 X{}.{} Y{} F{}\n", i % 97, i % 7, i * 31 % 1013, i % 5))
        .collect::<String>()
        .into_bytes()
}

fn cnc_update(part: &[u8]) -> BranchUpdate {
    BranchUpdate::new("cnc", "1.0", "0.1.0")
        .with_entry("program.nc", &made_up_program())
        .with_entry("part.json", part)
}

fn save_cnc(project: Project, part: &[u8]) -> Result<Project, eh3d_container::ContainerError> {
    project.save_branch(
        &Writer::for_app("cnc"),
        cnc_update(part),
        &SaveOptions::default(),
    )
}

/// The synthetic project with a program in `cnc/` and an attachment, written again at another
/// level of compression.
fn foreign_level(dir: &Path) -> std::path::PathBuf {
    let project = save_cnc(synthetic(dir), b"{\"v\":1}").unwrap();
    let sheet = dir.join("sheet.txt");
    std::fs::write(&sheet, made_up_program()).unwrap();
    let done = Attacher::new("pcb", "0.1.0")
        .attach_file(
            project,
            &sheet,
            &AttachRequest::new(),
            &SaveOptions::default(),
        )
        .unwrap();
    let path = done.project.path().to_path_buf();
    drop(done);
    let canonical = raw_entries(&path);
    repack(&path, 1, None);
    assert_ne!(
        raw_entries(&path)["cnc/program.nc"],
        canonical["cnc/program.nc"],
        "the other level must give other bytes, or the tests prove nothing"
    );
    path
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".eh3dtmp"))
        .collect()
}

#[test]
fn saving_one_branch_does_not_compress_the_other_entries_again() {
    let dir = tempdir().unwrap();
    let path = foreign_level(dir.path());
    let before = raw_entries(&path);
    let mut project = save(
        open(&path),
        "laser",
        BranchUpdate::new("laser", "1.0", "0.1.1").with_entry("logo.json", b"{\"power\":40}"),
        "2000-01-02T00:00:00Z",
    );
    let after = raw_entries(&path);
    let kept = before
        .keys()
        .filter(|n| !n.starts_with("laser/") && *n != "project.json" && *n != "integrity.json");
    let mut count = 0;
    for name in kept {
        assert_eq!(after.get(name), before.get(name), "{name}");
        count += 1;
    }
    assert!(count > 15, "{count}");
    assert!(after.keys().any(|n| n.starts_with("attachments/blobs/")));
    // the project that comes back is checked, and so is the file
    assert!(project.integrity_verified());
    assert!(project
        .diagnose(&Checks::default())
        .iter()
        .all(|f| !f.is_error()));
    assert!(open(&path).integrity_verified());
}

#[test]
fn a_file_of_the_saved_branch_that_did_not_change_is_not_compressed_again() {
    let dir = tempdir().unwrap();
    let path = foreign_level(dir.path());
    let before = raw_entries(&path);
    let mut project = save_cnc(open(&path), b"{\"v\":2}").unwrap();
    let after = raw_entries(&path);
    assert_eq!(after["cnc/program.nc"], before["cnc/program.nc"]);
    assert_ne!(after["cnc/part.json"], before["cnc/part.json"]);
    assert_eq!(
        project.read_entry("cnc", "part.json").unwrap(),
        b"{\"v\":2}"
    );
    assert_eq!(
        project.read_entry("cnc", "program.nc").unwrap(),
        made_up_program()
    );
}

#[test]
fn a_project_written_by_the_suite_keeps_its_one_form() {
    let dir = tempdir().unwrap();
    let path = save_cnc(synthetic(dir.path()), b"{\"v\":1}")
        .unwrap()
        .path()
        .to_path_buf();
    let first = std::fs::read(&path).unwrap();
    // the same content again, copied instead of compressed: the same file, byte for byte
    save_cnc(open(&path), b"{\"v\":1}").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), first);
    // and another writer of the same form gives bytes that a save leaves as they are
    repack(&path, 6, None);
    let anew = std::fs::read(&path).unwrap();
    save_cnc(open(&path), b"{\"v\":1}").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), anew);
}

#[test]
fn an_entry_damaged_after_the_project_was_opened_never_replaces_the_file() {
    let dir = tempdir().unwrap();
    for checked in [true, false] {
        let here = dir.path().join(format!("{checked}"));
        std::fs::create_dir(&here).unwrap();
        let path = synthetic(&here).path().to_path_buf();
        let project = if checked {
            open(&path)
        } else {
            open_unchecked(&path)
        };
        flip_in_entry(&path, "pcb/outline.json");
        let damaged = std::fs::read(&path).unwrap();
        assert!(save_cnc(project, b"{}").is_err(), "checked: {checked}");
        assert_eq!(std::fs::read(&path).unwrap(), damaged);
        assert_eq!(leftovers(&here), Vec::<String>::new());
    }
}

#[test]
fn an_entry_that_is_not_what_the_record_says_is_refused_by_a_save() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_path_buf();
    // the same size, valid compressed data and CRC, but not the content the record hashes
    let other = {
        let mut bytes = open(&path).read_entry("pcb", "outline.json").unwrap();
        let at = bytes.len() / 2;
        bytes[at] ^= 0x01;
        bytes
    };
    repack(&path, 6, Some(("pcb/outline.json", &other)));
    assert!(Project::open(&path, &OpenOptions::default()).is_err());
    let wrong = std::fs::read(&path).unwrap();
    let error = save_cnc(open_unchecked(&path), b"{}").unwrap_err();
    assert_eq!(error.code(), Some(PkgCode::Integrity));
    assert_eq!(std::fs::read(&path).unwrap(), wrong);
    assert_eq!(leftovers(dir.path()), Vec::<String>::new());
}
