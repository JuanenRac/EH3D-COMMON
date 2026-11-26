//! Projects that were tampered with, taking out that fails half way and files with awkward names. Every file here is made up.

mod support;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use eh3d_container::PkgCode;
use eh3d_project::appbranch::{conversion_report, BranchKit, Conversion, Identity};
use eh3d_project::attachments::{self, read_index};
use eh3d_project::{AttachRequest, Attacher, Checks, OpenOptions, Project, SaveOptions};
use serde_json::Value;
use support::*;
use tempfile::tempdir;

fn unchecked() -> OpenOptions {
    OpenOptions {
        verify_integrity: false,
        ..OpenOptions::default()
    }
}

/// Writes the archive at `path` again with `integrity.json` changed by `edit`.
fn forge_record(path: &Path, edit: impl Fn(&mut Value)) {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().expect("entry name").into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == "integrity.json" {
            let mut record: Value = serde_json::from_slice(&bytes).unwrap();
            edit(&mut record);
            bytes = canon(&record);
        }
        entries.push((name, bytes));
    }
    drop(archive);
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, bytes) in entries {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
}

/// Writes a wrong CRC in the list of entries of the archive for the entry `name`.
fn wrong_crc(path: &Path, name: &str) {
    let mut bytes = fs::read(path).unwrap();
    let mut changed = 0;
    for at in 0..bytes.len().saturating_sub(46 + name.len()) {
        if bytes[at..at + 4] == *b"PK\x01\x02"
            && bytes[at + 46..at + 46 + name.len()] == *name.as_bytes()
        {
            bytes[at + 16] ^= 0xff;
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "{name}");
    fs::write(path, bytes).unwrap();
}

#[test]
fn a_size_that_the_record_lies_about_is_refused_with_and_without_the_check() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_owned();
    let whole = fs::read(&path).unwrap();
    for size in [0, 1, u64::from(u32::MAX), 1u64 << 52] {
        fs::write(&path, &whole).unwrap();
        forge_record(&path, |record| {
            record["files"]["cnc/part.json"]["size"] = size.into();
        });
        for options in [unchecked(), OpenOptions::default()] {
            let error = Project::open(&path, &options).err().unwrap();
            assert_eq!(error.code(), Some(PkgCode::Integrity), "{size}: {error}");
        }
    }
}

#[test]
fn a_branch_that_cannot_be_taken_out_whole_leaves_no_half_file() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_owned();
    wrong_crc(&path, "cnc/part.json");
    assert!(Project::open(&path, &OpenOptions::default()).is_err());
    let mut project = Project::open(&path, &unchecked()).unwrap();
    let out = dir.path().join("out");
    assert!(project.extract_branch("cnc", &out).is_err());
    assert!(!out.join("part.json").exists());
}

#[test]
fn an_imported_file_with_the_name_of_the_report_is_not_written_over() {
    const KIT: BranchKit<'static> = BranchKit::new("laser", "1.0", "laser.json", "0.1.0");
    for name in ["report.json", "Report.JSON"] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("work.eh3d");
        let bytes = b"{\"drawn\":\"by another program\"}".to_vec();
        let report = conversion_report(
            "tests",
            "0.1.0",
            "json",
            name,
            &bytes,
            Conversion::default(),
        );
        let mut told = String::new();
        KIT.import(
            &path,
            &Identity::default(),
            bytes.clone(),
            &report,
            |_, original| {
                told = original.to_owned();
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(told, format!("origin/original-{name}"));
        let (_, copy, warnings) = KIT.open(&path).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(copy.file(&told), Some(&bytes[..]));
        assert_eq!(copy.report().unwrap().unwrap().original, Some(told.clone()));
    }
}

#[test]
fn awkward_names_of_attachments_leave_a_sound_project() {
    let dir = tempdir().unwrap();
    let mut project = new_project(dir.path());
    let attacher = Attacher::new("pcb", "0.1.0");
    let long_extension = format!("{}.{}", "a".repeat(149), "b".repeat(30));
    let names = [
        long_extension.as_str(),
        "..\\..\\CON.txt",
        "  . ",
        "-rf",
        "\u{202e}fdp.exe",
        "日本語のとても長い名前の資料日本語のとても長い名前の資料日本語のとても長い名前の資料日本語のとても長い名前の資料.pdf",
    ];
    for (n, name) in names.iter().enumerate() {
        let source = dir.path().join(format!("in{n}"));
        fs::write(&source, format!("content {n}")).unwrap();
        project = attacher
            .attach_file(
                project,
                &source,
                &AttachRequest::new().name(name),
                &SaveOptions::default(),
            )
            .unwrap()
            .project;
    }
    let index = read_index(&mut project).unwrap();
    assert_eq!(index.attachments.len(), names.len());
    let findings = project.diagnose(&Checks::default());
    assert!(findings.iter().all(|f| !f.is_error()), "{findings:?}");
    let out = dir.path().join("out");
    for a in &index.attachments {
        let taken = attachments::extract(
            &mut project,
            a.id(),
            &out,
            attachments::Consent::Given,
            &attacher.limits,
        )
        .unwrap();
        assert_eq!(taken.parent(), Some(out.as_path()), "{}", a.name);
    }
}

#[cfg(unix)]
#[test]
fn an_attachment_of_gigabytes_is_refused_before_it_is_read() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let path = project.path().to_owned();
    let before = fs::read(&path).unwrap();
    // a file with a hole: four gibibytes announced, nothing on the disk
    let huge = dir.path().join("huge.bin");
    File::create(&huge).unwrap().set_len(4 << 30).unwrap();
    let mut attacher = Attacher::new("pcb", "0.1.0");
    assert!(attacher
        .attach_file(
            project,
            &huge,
            &AttachRequest::new(),
            &SaveOptions::default()
        )
        .is_err());
    assert!(attacher.limits.apply("attachments.max_mb", "4096"));
    let project = Project::open(&path, &OpenOptions::default()).unwrap();
    let error = attacher
        .attach_file(
            project,
            &huge,
            &AttachRequest::new(),
            &SaveOptions::default(),
        )
        .err()
        .unwrap();
    assert!(
        matches!(
            error,
            eh3d_project::AttachError::ProjectFull { .. }
                | eh3d_project::AttachError::TooLarge { .. }
        ),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
}
