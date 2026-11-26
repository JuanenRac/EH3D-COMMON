//! The attachments of a project: stored once whatever uses them, large ones in parts, the other branches untouched, dangerous ones
//! taken out only with an agreement, links that download nothing, hostile records that cannot leave a folder or pass for harmless,
//! and the limits. Every file here is made up.

mod support;

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use eh3d_container::{sha256_hex, PkgCode};
use eh3d_project::attachments::{
    self, matches_file, read_index, Storage, DEFAULT_MAX_ATTACHMENT_BYTES, INDEX_FILE,
};
use eh3d_project::spec::{eh3d, ATTACHMENTS_BRANCH};
use eh3d_project::{
    suiteview, AttachError, AttachRequest, Attacher, AttachmentKind, AttachmentLimits,
    AttachmentSource, BranchUpdate, Checks, ComponentLink, ComponentQuery, Consent, Danger,
    OpenOptions, Project, SaveOptions, Seal, Writer,
};
use serde_json::json;
use support::*;
use tempfile::tempdir;

fn attacher() -> Attacher {
    Attacher::new("pcb", "0.1.0")
}

fn small_parts(part_bytes: u64) -> Attacher {
    attacher().with_limits(AttachmentLimits::default().with_part_bytes(part_bytes))
}

fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Bytes that do not repeat in an obvious way, from a fixed rule.
fn made_up(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i.wrapping_mul(31) ^ (i >> 3)) as u8)
        .collect()
}

fn open(path: &Path) -> Project {
    Project::open(path, &OpenOptions::default()).unwrap()
}

fn attach(project: Project, source: &Path, request: &AttachRequest) -> attachments::Attached {
    attacher()
        .attach_file(project, source, request, &SaveOptions::default())
        .unwrap()
}

fn zip_of(names: &[&str]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for name in names {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn blob_names(project: &Project) -> Vec<String> {
    project
        .branch(ATTACHMENTS_BRANCH)
        .map(|b| {
            b.files
                .keys()
                .filter(|n| n.starts_with("blobs/"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn assert_sound(project: &mut Project) {
    let findings = project.diagnose(&Checks::default());
    let errors: Vec<_> = findings.iter().filter(|f| f.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_data_sheet_that_many_pieces_use_is_stored_once_and_the_other_branches_keep_their_bytes() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_path_buf();
    let before = raw_entries(&path);
    let sheet = file(dir.path(), "NE555.pdf", b"%PDF-1.4 made up sheet");
    let request = AttachRequest::new()
        .title("en", "Timer data sheet")
        .component(ComponentLink::designator("U3"));
    let first = attach(open(&path), &sheet, &request);
    assert!(first.written && !first.already_there);
    assert_eq!(first.id, sha256_hex(b"%PDF-1.4 made up sheet"));

    // the same content under another name, for another piece
    let copy = file(dir.path(), "copy of it.pdf", b"%PDF-1.4 made up sheet");
    let more = AttachRequest::new().component(ComponentLink::designator("U4"));
    let second = attach(first.project, &copy, &more);
    assert!(second.written && second.already_there);
    assert_eq!(second.id, first.id);

    let mut project = second.project;
    let index = read_index(&mut project).unwrap();
    assert_eq!(index.attachments.len(), 1);
    let record = &index.attachments[0];
    assert_eq!(record.name, "NE555.pdf");
    assert_eq!(record.kind(), AttachmentKind::Pdf);
    assert_eq!(record.danger(), None);
    assert_eq!(record.storage, Storage::Inside);
    assert_eq!(record.components.len(), 2);
    assert_eq!(blob_names(&project).len(), 1);
    assert_sound(&mut project);

    // nothing new: the file is not written again
    let bytes = std::fs::read(&path).unwrap();
    let again = attach(project, &sheet, &request);
    assert!(!again.written && again.already_there);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let after = raw_entries(&path);
    for (name, entry) in &before {
        if name != "project.json" && name != "integrity.json" {
            assert_eq!(after.get(name), Some(entry), "{name}");
        }
    }
}

#[test]
fn a_large_file_is_stored_in_parts_and_read_back_whole() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_path_buf();
    let content = made_up(3_500);
    let video = file(dir.path(), "assembly.mp4", &content);
    let done = small_parts(1_000)
        .attach_file(
            open(&path),
            &video,
            &AttachRequest::new(),
            &SaveOptions::default(),
        )
        .unwrap();
    let mut project = done.project;
    assert_eq!(blob_names(&project).len(), 4);
    assert_sound(&mut project);

    let source = AttachmentSource::open(&path, &done.id).unwrap();
    // a viewer may keep it and read it from another thread
    fn kept_by_a_viewer<T: Send + Sync + 'static>(_: &T) {}
    kept_by_a_viewer(&source);
    assert_eq!(source.name(), "assembly.mp4");
    assert_eq!(source.len(), 3_500);
    assert_eq!(source.record().kind(), AttachmentKind::Video);
    // as often as needed, and in small pieces
    for _ in 0..2 {
        let mut reader = source.reader().unwrap();
        let mut back = Vec::new();
        let mut piece = [0u8; 7];
        loop {
            let n = reader.read(&mut piece).unwrap();
            if n == 0 {
                break;
            }
            back.extend_from_slice(&piece[..n]);
        }
        assert_eq!(back, content);
    }
    let out = attachments::extract(
        &mut project,
        &done.id,
        &dir.path().join("out"),
        Consent::NotAsked,
        &AttachmentLimits::default(),
    )
    .unwrap();
    assert_eq!(std::fs::read(out).unwrap(), content);

    // a program that knows nothing of attachments saves its own branch: every part stays as it was
    let before = raw_entries(&path);
    let project = save(
        project,
        "cnc",
        BranchUpdate::new("cnc", "1.0", "0.1.0").with_entry("part.json", b"{}"),
        "2000-01-02T00:00:00Z",
    );
    let after = raw_entries(&path);
    for (name, entry) in before.iter().filter(|(n, _)| n.starts_with("attachments/")) {
        assert_eq!(after.get(name), Some(entry), "{name}");
    }
    // the parts are listed as attachments, never offered to a program as files to take
    let view = suiteview::view(project.path(), "cnc").unwrap();
    assert!(view.branch(ATTACHMENTS_BRANCH).is_some());
    assert!(view.takeable.iter().all(|(b, _)| b != ATTACHMENTS_BRANCH));

    // taking it out of the project takes its parts too
    let gone = attacher()
        .detach(project, &done.id[..12], &SaveOptions::default())
        .unwrap();
    assert!(blob_names(&gone.project).is_empty());
    let mut project = gone.project;
    assert!(read_index(&mut project).unwrap().attachments.is_empty());
    assert_sound(&mut project);
}

#[test]
fn an_empty_file_and_a_file_of_exactly_one_part_are_attached() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path()).path().to_path_buf();
    let mut project = open(&path);
    for (name, len) in [("empty.txt", 0usize), ("exact.bin2", 100)] {
        let source = file(dir.path(), name, &made_up(len));
        let done = small_parts(100)
            .attach_file(
                project,
                &source,
                &AttachRequest::new(),
                &SaveOptions::default(),
            )
            .unwrap();
        let mut back = Vec::new();
        AttachmentSource::open(&path, &done.id)
            .unwrap()
            .reader()
            .unwrap()
            .read_to_end(&mut back)
            .unwrap();
        assert_eq!(back, made_up(len), "{name}");
        project = done.project;
    }
    assert_eq!(blob_names(&project).len(), 2);
    assert_sound(&mut project);
}

#[test]
fn the_limits_are_kept_and_a_refusal_leaves_the_file_as_it_was() {
    assert_eq!(
        AttachmentLimits::default().max_attachment_bytes,
        DEFAULT_MAX_ATTACHMENT_BYTES
    );
    assert_eq!(DEFAULT_MAX_ATTACHMENT_BYTES, 1 << 30);
    // a reader of the format that knows nothing of attachments accepts every part, and the whole stays under four gigabytes
    let spec = eh3d();
    assert!(
        AttachmentLimits::default().part_bytes
            <= spec.limits.max_entry_bytes_for("attachments/blobs/x/00000")
    );
    assert!(spec.limits.max_total_bytes < 4 << 30);
    let mut limits = AttachmentLimits::default();
    assert!(limits.apply("attachments.max_mb", " 1 "));
    assert_eq!(limits.max_attachment_bytes, 1 << 20);
    for (key, value) in [
        ("attachments.max_mb", "0"),
        ("attachments.max_mb", "4097"),
        ("attachments.max_mb", "-1"),
        ("attachments.other", "5"),
    ] {
        assert!(!limits.apply(key, value), "{key}={value}");
    }
    assert_eq!(limits.max_attachment_bytes, 1 << 20);

    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_path_buf();
    let bytes = std::fs::read(&path).unwrap();
    let big = file(dir.path(), "big.bin2", &vec![0u8; (1 << 20) + 1]);
    let error = attacher()
        .with_limits(limits)
        .attach_file(
            open(&path),
            &big,
            &AttachRequest::new(),
            &SaveOptions::default(),
        )
        .unwrap_err();
    assert!(matches!(error, AttachError::TooLarge { .. }), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // the whole project has a limit too
    let mut spec = eh3d();
    spec.limits.max_total_bytes = 17 << 20;
    let narrow = Project::open(
        &path,
        &OpenOptions {
            spec,
            verify_integrity: true,
        },
    )
    .unwrap();
    let two = file(dir.path(), "two.bin2", &vec![0u8; 2 << 20]);
    let error = attacher()
        .attach_file(narrow, &two, &AttachRequest::new(), &SaveOptions::default())
        .unwrap_err();
    assert!(matches!(error, AttachError::ProjectFull { .. }), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // an attachment above the limit of whoever takes it out is not written
    let ok = file(dir.path(), "ok.bin2", &vec![0u8; 2 << 20]);
    let done = attach(open(&path), &ok, &AttachRequest::new());
    let mut project = done.project;
    let out = dir.path().join("out");
    let error =
        attachments::extract(&mut project, &done.id, &out, Consent::Given, &limits).unwrap_err();
    assert!(matches!(error, AttachError::TooLarge { .. }), "{error}");
    assert!(!out.join("ok.bin2").exists());
}

#[test]
fn dangerous_files_are_kept_marked_and_taken_out_only_with_an_agreement() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path()).path().to_path_buf();
    let cases: [(&str, Vec<u8>, Danger); 6] = [
        (
            "setup.exe",
            b"MZ\x90\x00made up".to_vec(),
            Danger::Executable,
        ),
        (
            "flash.sh",
            b"#!/bin/sh\necho made up\n".to_vec(),
            Danger::Script,
        ),
        ("sheet.xlsm", zip_of(&["xl/workbook.xml"]), Danger::Macros),
        (
            "hidden.docx",
            zip_of(&["word/vbaProject.bin"]),
            Danger::Macros,
        ),
        ("sources.zip", zip_of(&["a.txt"]), Danger::Archive),
        (
            "page.html",
            b"<p>made up</p>".to_vec(),
            Danger::ActiveContent,
        ),
    ];
    let mut project = open(&path);
    let out = dir.path().join("out");
    let limits = AttachmentLimits::default();
    for (name, bytes, danger) in &cases {
        let source = file(dir.path(), name, bytes);
        let done = attach(project, &source, &AttachRequest::new());
        project = done.project;
        let index = read_index(&mut project).unwrap();
        let record = index.find(&done.id).unwrap();
        assert_eq!(record.danger(), Some(*danger), "{name}");
        assert!(record.needs_confirmation());
        let error = attachments::extract(&mut project, &done.id, &out, Consent::NotAsked, &limits)
            .unwrap_err();
        assert!(
            matches!(error, AttachError::NeedsConfirmation { .. }),
            "{name}: {error}"
        );
        assert!(!out.join(name).exists(), "{name}");
        let written =
            attachments::extract(&mut project, &done.id, &out, Consent::Given, &limits).unwrap();
        assert_eq!(&std::fs::read(&written).unwrap(), bytes, "{name}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&written).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0, "{name} is not executable");
        }
    }
    // a harmless name does not hide a program on disk
    let source = file(dir.path(), "real.exe", b"made up, no magic");
    let done = attach(project, &source, &AttachRequest::new().name("manual.pdf"));
    let mut project = done.project;
    let index = read_index(&mut project).unwrap();
    assert_eq!(
        index.find(&done.id).unwrap().danger(),
        Some(Danger::Executable)
    );
    assert_sound(&mut project);
}

/// A branch of attachments written by hand, as a hostile file would have it.
fn crafted(dir: &Path, name: &str, announced: &[u8], stored: &[u8]) -> (PathBuf, String) {
    let project = new_project(dir);
    let sha256 = sha256_hex(announced);
    let index = canon(&json!({
        "schema": 1,
        "attachments": [{
            "sha256": sha256, "name": name, "size": stored.len(), "kind": "text", "storage": "inside",
        }],
    }));
    let update = BranchUpdate::new(ATTACHMENTS_BRANCH, "1.0", "9.9.9")
        .with_entry(INDEX_FILE, &index)
        .with_entry(&format!("blobs/{sha256}/00000"), stored);
    let project = project
        .save_branch(&Writer::new("other", &[]), update, &SaveOptions::default())
        .unwrap();
    (project.path().to_path_buf(), sha256)
}

#[test]
fn a_hostile_record_cannot_leave_the_folder_or_pass_for_harmless() {
    let limits = AttachmentLimits::default();

    // a name that is a path
    let dir = tempdir().unwrap();
    let (path, id) = crafted(dir.path(), "../../escape.txt", b"made up", b"made up");
    let mut project = open(&path);
    let findings = project.diagnose(&Checks::default());
    assert!(findings.iter().any(|f| f.code == PkgCode::BranchRecord));
    let out = dir.path().join("a").join("b");
    let written =
        attachments::extract(&mut project, &id, &out, Consent::NotAsked, &limits).unwrap();
    assert_eq!(written, out.join("escape.txt"));
    assert!(!dir.path().join("escape.txt").exists());

    // a script behind a harmless name, with no danger said: the content is looked at
    let dir = tempdir().unwrap();
    let script = b"#!/bin/sh\nmade up\n";
    let (path, id) = crafted(dir.path(), "notes.txt", script, script);
    let out = dir.path().join("out");
    let error =
        attachments::extract(&mut open(&path), &id, &out, Consent::NotAsked, &limits).unwrap_err();
    assert!(
        matches!(error, AttachError::NeedsConfirmation { .. }),
        "{error}"
    );
    assert!(!out.join("notes.txt").exists());

    // a file that is not the one announced
    let dir = tempdir().unwrap();
    let (path, id) = crafted(dir.path(), "notes.txt", b"announced", b"another!!");
    let mut project = open(&path);
    let findings = project.diagnose(&Checks::default());
    assert!(findings.iter().any(|f| f.code == PkgCode::BranchRecord));
    let out = dir.path().join("out");
    assert!(attachments::extract(&mut project, &id, &out, Consent::Given, &limits).is_err());
    assert!(!out.join("notes.txt").exists(), "no partial file is left");
    let mut back = Vec::new();
    let source = AttachmentSource::open(&path, &id).unwrap();
    let mut reader = source.reader().unwrap();
    assert!(reader.read_to_end(&mut back).is_err());
    assert!(back.is_empty(), "nothing of a wrong file is handed over");
    assert!(reader.read(&mut [0u8; 4]).is_err(), "it stays wrong");
}

#[test]
fn a_link_keeps_an_address_and_a_hash_and_downloads_nothing() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path()).path().to_path_buf();
    let content = b"made up manual";
    let sha = sha256_hex(content);
    let done = attacher()
        .attach_link(
            open(&path),
            "https://example.invalid/docs/manual.pdf?v=2",
            &sha.to_uppercase(),
            Some(content.len() as u64),
            &AttachRequest::new().component(ComponentLink::part(Some("Example"), "AB-1")),
            &SaveOptions::default(),
        )
        .unwrap();
    assert_eq!(done.id, sha);
    let mut project = done.project;
    assert!(blob_names(&project).is_empty());
    assert_sound(&mut project);
    let index = read_index(&mut project).unwrap();
    let record = index.find(&sha[..8]).unwrap();
    assert!(record.is_link());
    assert_eq!(record.name, "manual.pdf");
    assert!(index.find(&sha[..7]).is_err(), "too short to name one");
    assert!(matches!(
        index.find(&"0".repeat(10)),
        Err(AttachError::NotFound(_))
    ));
    assert!(matches!(
        AttachmentSource::open(&path, &sha),
        Err(AttachError::LinkOnly { .. })
    ));
    let error = attachments::extract(
        &mut project,
        &sha,
        &dir.path().join("out"),
        Consent::Given,
        &AttachmentLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(error, AttachError::LinkOnly { .. }), "{error}");

    // what a person got from the address can be checked
    let good = file(dir.path(), "got.pdf", content);
    let bad = file(dir.path(), "other.pdf", b"something else");
    assert!(matches_file(record, &good).unwrap());
    assert!(!matches_file(record, &bad).unwrap());
    assert_eq!(
        attachments::hash_file(&good).unwrap(),
        (sha.clone(), content.len() as u64)
    );

    // putting the file itself makes the link a file inside, with its address kept
    let inside = attach(project, &good, &AttachRequest::new());
    assert!(inside.already_there && inside.written);
    let mut project = inside.project;
    let index = read_index(&mut project).unwrap();
    assert_eq!(index.attachments.len(), 1);
    assert_eq!(index.attachments[0].storage, Storage::Inside);
    assert!(index.attachments[0].url.is_some());
    assert_eq!(index.attachments[0].components.len(), 1);
    assert_sound(&mut project);

    for url in [
        "ftp://example.invalid/a",
        "file:///etc/passwd",
        "https://",
        "https://someone@example.invalid/a",
        "https://example.invalid/a b",
        "javascript:alert(1)",
    ] {
        let result = attacher().attach_link(
            open(&path),
            url,
            &"b".repeat(64),
            None,
            &AttachRequest::new(),
            &SaveOptions::default(),
        );
        assert!(result.is_err(), "{url}");
    }
    let result = attacher().attach_link(
        open(&path),
        "https://example.invalid/a",
        "abc",
        None,
        &AttachRequest::new(),
        &SaveOptions::default(),
    );
    assert!(result.is_err(), "a link needs the hash");
}

#[test]
fn from_a_piece_of_the_design_its_documents_are_found_the_most_specific_first() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path()).path().to_path_buf();
    let mut project = open(&path);
    let add = |name: &str, link: ComponentLink, project: Project| {
        let source = file(dir.path(), name, name.as_bytes());
        attach(project, &source, &AttachRequest::new().component(link)).project
    };
    project = add("generic.pdf", ComponentLink::of_value("10k"), project);
    project = add("series.pdf", ComponentLink::part(None, "AB-1"), project);
    project = add(
        "maker.pdf",
        ComponentLink::part(Some("Example"), "AB-1"),
        project,
    );
    project = add(
        "other-maker.pdf",
        ComponentLink::part(Some("Elsewhere"), "AB-1"),
        project,
    );
    project = add("this-one.pdf", ComponentLink::designator("R5"), project);
    project = add("mega.pdf", ComponentLink::of_value("1M"), project);
    let index = read_index(&mut project).unwrap();
    let names = |query: ComponentQuery<'_>| -> Vec<String> {
        index
            .for_component(&query)
            .iter()
            .map(|a| a.name.clone())
            .collect()
    };
    let all = ComponentQuery::new()
        .reference("r5")
        .value("10k")
        .part_number("ab-1")
        .manufacturer("EXAMPLE");
    assert_eq!(
        names(all),
        ["this-one.pdf", "maker.pdf", "series.pdf", "generic.pdf"]
    );
    assert_eq!(
        names(ComponentQuery::new().value("1m")),
        Vec::<String>::new()
    );
    assert_eq!(names(ComponentQuery::new().value("1M")), ["mega.pdf"]);
    assert_eq!(names(ComponentQuery::new()), Vec::<String>::new());

    // the pieces of one attachment can be changed later
    let id = index.for_component(&ComponentQuery::new().reference("R5"))[0]
        .sha256
        .clone();
    let changed = attacher()
        .set_components(
            project,
            &id,
            &[ComponentLink::designator("R6")],
            &[ComponentLink::designator("R5")],
            &SaveOptions::default(),
        )
        .unwrap();
    let mut project = changed.project;
    let index = read_index(&mut project).unwrap();
    assert!(index
        .for_component(&ComponentQuery::new().reference("R5"))
        .is_empty());
    assert_eq!(
        index
            .for_component(&ComponentQuery::new().reference("R6"))
            .len(),
        1
    );
    // an empty text is not a piece of anything
    let source = file(dir.path(), "x.pdf", b"x");
    let result = attacher().attach_file(
        project,
        &source,
        &AttachRequest::new().component(ComponentLink::designator("  ")),
        &SaveOptions::default(),
    );
    assert!(result.is_err());
}

#[test]
fn a_secret_key_a_sealed_project_and_a_newer_format_are_refused() {
    let dir = tempdir().unwrap();
    let path = synthetic(dir.path()).path().to_path_buf();
    let bytes = std::fs::read(&path).unwrap();
    // an old project without attachments has none
    assert!(read_index(&mut open(&path)).unwrap().attachments.is_empty());

    let header = ["eh3d-ed25519-", "secret-v1\n"].concat();
    let key = file(dir.path(), "signing.txt", header.as_bytes());
    let result = attacher().attach_file(
        open(&path),
        &key,
        &AttachRequest::new(),
        &SaveOptions::default(),
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let note = file(dir.path(), "note.txt", b"made up");
    let sealed = open(&path)
        .seal_project(
            Seal {
                by: "tests".into(),
                at: "2000-01-01T00:00:00Z".into(),
                reason: None,
            },
            &SaveOptions::default(),
        )
        .unwrap();
    let error = attacher()
        .attach_file(
            sealed,
            &note,
            &AttachRequest::new(),
            &SaveOptions::default(),
        )
        .unwrap_err();
    assert!(
        matches!(&error, AttachError::Container(e) if e.code() == Some(PkgCode::Sealed)),
        "{error}"
    );

    let dir = tempdir().unwrap();
    let update = BranchUpdate::new(ATTACHMENTS_BRANCH, "2.0", "9.9.9")
        .with_entry(INDEX_FILE, b"{\"schema\":2}");
    let newer = new_project(dir.path())
        .save_branch(&Writer::new("later", &[]), update, &SaveOptions::default())
        .unwrap();
    let note = file(dir.path(), "note.txt", b"made up");
    let error = attacher()
        .attach_file(newer, &note, &AttachRequest::new(), &SaveOptions::default())
        .unwrap_err();
    assert!(
        matches!(&error, AttachError::Container(e) if e.code() == Some(PkgCode::BranchVersion)),
        "{error}"
    );
}

#[test]
fn the_same_attachments_give_the_same_bytes() {
    let mut results = Vec::new();
    for _ in 0..2 {
        let dir = tempdir().unwrap();
        let path = synthetic(dir.path()).path().to_path_buf();
        let a = file(dir.path(), "a.step", &made_up(5_000));
        let b = file(dir.path(), "b.pdf", &made_up(10));
        let request = AttachRequest::new().component(ComponentLink::designator("J1"));
        let done = attach(open(&path), &a, &request);
        attach(done.project, &b, &request);
        results.push(std::fs::read(&path).unwrap());
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn a_kind_the_suite_does_not_know_is_taken_out_only_with_an_agreement() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path()).path().to_path_buf();
    let source = file(dir.path(), "dump.made-up", b"made up bytes");
    let done = attach(open(&path), &source, &AttachRequest::new());
    let mut project = done.project;
    let index = read_index(&mut project).unwrap();
    let record = index.find(&done.id).unwrap();
    assert_eq!(
        record.danger(),
        None,
        "unknown is not a danger of the record"
    );
    assert!(record.needs_confirmation());
    let out = dir.path().join("out");
    let limits = AttachmentLimits::default();
    let error =
        attachments::extract(&mut project, &done.id, &out, Consent::NotAsked, &limits).unwrap_err();
    assert!(
        matches!(error, AttachError::NeedsConfirmation { danger: None, .. }),
        "{error}"
    );
    assert!(!out.join("dump.made-up").exists());
    let written =
        attachments::extract(&mut project, &done.id, &out, Consent::Given, &limits).unwrap();
    assert_eq!(std::fs::read(written).unwrap(), b"made up bytes");
}
