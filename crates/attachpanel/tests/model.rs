//! The panel with no screen: what it lists, what it does and what it asks first. Every file here is made up.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eh3d_attachpanel::model::{component_link, link_label, Ask};
use eh3d_attachpanel::texts::{Lang, Text};
use eh3d_attachpanel::{LinkForm, Message, Model, Piece, Program, ProjectSource, Request};
use eh3d_container::AtomicOptions;
use eh3d_project::ed25519::Ed25519Signer;
use eh3d_project::{Danger, NewProject, OpenOptions, Project, SaveOptions};
use eh3d_viewer::Source;
use tempfile::tempdir;

fn new_project(dir: &Path) -> PathBuf {
    let path = dir.join("made-up.eh3d");
    Project::create(
        &path,
        NewProject {
            id: "0f6d3c1a-9b1e-4c53-9b60-3a8f1d2c7e11".to_owned(),
            name: "made up".to_owned(),
            units: "mm".to_owned(),
            created: Some("2000-01-01T00:00:00Z".to_owned()),
        },
        &OpenOptions::default(),
        &AtomicOptions::default(),
    )
    .unwrap();
    path
}

fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn model() -> Model {
    Model::new(Program::new("cnc", "0.1.0"))
}

fn wait(model: &mut Model) {
    let until = Instant::now() + Duration::from_secs(60);
    while model.busy() {
        assert!(Instant::now() < until, "the work did not end");
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
}

fn run(model: &mut Model, request: Request) {
    model.request(request);
    wait(model);
}

fn id_of(model: &Model, name: &str) -> String {
    model
        .items()
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("{name} is not listed: {:?}", model.message()))
        .id
        .clone()
}

#[test]
fn a_file_is_attached_listed_shown_extracted_and_removed() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    // nothing happens without a project
    panel.request(Request::AttachFile(dir.path().join("x")));
    assert!(!panel.busy() && panel.pending().is_none() && panel.message().is_none());
    panel.set_project(Some(&project));
    assert!(panel.items().is_empty() && panel.error().is_none());

    let sheet = file(dir.path(), "NE555.pdf", b"%PDF-1.4 made up");
    run(&mut panel, Request::AttachFile(sheet.clone()));
    assert_eq!(panel.message(), Some(&Message::Attached));
    assert!(panel.take_wrote() && !panel.take_wrote());
    assert_eq!(panel.items().len(), 1);
    let id = id_of(&panel, "NE555.pdf");
    assert_eq!(panel.selected().map(|i| i.id.clone()), Some(id.clone()));
    assert!(!panel.items()[0].needs_confirmation);

    run(&mut panel, Request::AttachFile(sheet));
    assert_eq!(panel.message(), Some(&Message::AlreadyThere));
    assert_eq!(panel.items().len(), 1);
    assert!(!panel.take_wrote(), "nothing new was written");

    // the viewer reads it from inside the project
    let source = ProjectSource(panel.source(&id).unwrap().unwrap());
    let mut bytes = Vec::new();
    source.reader().unwrap().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"%PDF-1.4 made up");
    assert_eq!(
        (source.name(), source.declared_danger()),
        ("NE555.pdf", None)
    );

    // a harmless file comes out at once, and never over a file that is there
    let out = dir.path().join("out");
    run(&mut panel, Request::Extract(id.clone(), out.clone()));
    assert_eq!(
        panel.message(),
        Some(&Message::Extracted(out.join("NE555.pdf")))
    );
    run(&mut panel, Request::Extract(id.clone(), out.clone()));
    assert!(matches!(panel.message(), Some(Message::Failed(_))));

    // removing always asks; saying no changes nothing
    panel.request(Request::Remove(id.clone()));
    assert!(!panel.busy());
    let question = panel.pending().expect("a question");
    assert_eq!(
        question.lines,
        [Ask::Remove {
            name: "NE555.pdf".to_owned(),
            components: 0
        }]
    );
    for lang in Lang::ALL {
        assert!(
            question.lines[0].text(lang).contains("NE555.pdf"),
            "{lang:?}"
        );
    }
    panel.cancel();
    panel.confirm();
    assert!(!panel.busy() && panel.items().len() == 1);
    panel.request(Request::Remove(id));
    panel.confirm();
    wait(&mut panel);
    assert_eq!(panel.message(), Some(&Message::Removed));
    assert!(panel.items().is_empty() && panel.selected().is_none() && panel.take_wrote());
}

#[test]
fn dangerous_and_unknown_files_are_marked_and_come_out_only_after_a_yes() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&project));
    run(
        &mut panel,
        Request::AttachFile(file(dir.path(), "tool.py", b"made up")),
    );
    assert_eq!(panel.message(), Some(&Message::Marked(Danger::Script)));
    run(
        &mut panel,
        Request::AttachFile(file(dir.path(), "dump.made-up", b"made up bytes")),
    );
    assert_eq!(panel.message(), Some(&Message::Attached));
    for (name, danger) in [("tool.py", Some(Danger::Script)), ("dump.made-up", None)] {
        let id = id_of(&panel, name);
        assert!(panel.item(&id).unwrap().needs_confirmation, "{name}");
        let out = dir.path().join("out");
        panel.request(Request::Extract(id.clone(), out.clone()));
        assert!(!panel.busy(), "{name}");
        assert_eq!(
            panel.pending().unwrap().lines,
            [Ask::Care {
                name: name.to_owned(),
                danger
            }]
        );
        assert!(!out.join(name).exists());
        panel.confirm();
        wait(&mut panel);
        assert_eq!(panel.message(), Some(&Message::Extracted(out.join(name))));
    }
}

#[test]
fn a_settings_file_with_a_token_does_not_go_into_a_project() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let before = fs::read(&project).unwrap();
    let mut panel = model();
    panel.set_project(Some(&project));
    let settings = file(dir.path(), "pnp.cfg", b"server_token = made-up\n");
    run(&mut panel, Request::AttachFile(settings));
    assert_eq!(
        panel.message(),
        Some(&Message::HoldsCredential("pnp.cfg".to_owned()))
    );
    assert!(panel.items().is_empty() && !panel.take_wrote());
    assert_eq!(fs::read(&project).unwrap(), before);
    // an empty value is not a credential
    let empty = file(dir.path(), "empty.cfg", b"server_token =\n");
    run(&mut panel, Request::AttachFile(empty));
    assert_eq!(panel.message(), Some(&Message::Attached));
}

#[test]
fn components_are_linked_and_the_list_is_filtered_by_the_part() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&project));
    for name in ["a.pdf", "b.pdf"] {
        run(
            &mut panel,
            Request::AttachFile(file(dir.path(), name, name.as_bytes())),
        );
    }
    let a = id_of(&panel, "a.pdf");
    assert_eq!(component_link(LinkForm::Designator, "  ", ""), None);
    let r5 = component_link(LinkForm::Designator, " R5 ", "ignored").unwrap();
    let part = component_link(LinkForm::PartNumber, "NE555", "TI").unwrap();
    assert_eq!(
        (link_label(&r5), link_label(&part)),
        ("R5".into(), "TI NE555".into())
    );
    run(&mut panel, Request::AddComponent(a.clone(), r5.clone()));
    assert_eq!(panel.message(), Some(&Message::Changed));
    run(&mut panel, Request::AddComponent(a.clone(), part));
    assert_eq!(panel.item(&a).unwrap().components.len(), 2);

    let piece = Piece {
        reference: Some("r5".to_owned()),
        ..Piece::default()
    };
    panel.set_filter(Some(piece));
    let names: Vec<&str> = panel.visible().iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["a.pdf"]);
    panel.set_filter(None);
    assert_eq!(panel.visible().len(), 2);

    run(&mut panel, Request::RemoveComponent(a.clone(), r5));
    assert_eq!(panel.item(&a).unwrap().components.len(), 1);
    panel.request(Request::Remove(a));
    assert_eq!(
        panel.pending().unwrap().lines,
        [Ask::Remove {
            name: "a.pdf".to_owned(),
            components: 1
        }]
    );
}

#[test]
fn a_link_keeps_only_the_address_and_has_nothing_to_show_or_extract() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&project));
    run(
        &mut panel,
        Request::AttachLink {
            url: "https://example.invalid/manual.pdf".to_owned(),
            sha256: "a".repeat(64),
            size: Some(10),
        },
    );
    assert_eq!(panel.message(), Some(&Message::Attached));
    let id = id_of(&panel, "manual.pdf");
    assert!(panel.item(&id).unwrap().is_link);
    assert!(panel.source(&id).is_none());
    run(&mut panel, Request::Extract(id, dir.path().join("out")));
    assert!(matches!(panel.message(), Some(Message::Failed(_))));
    run(
        &mut panel,
        Request::AttachLink {
            url: "file:///etc/passwd".to_owned(),
            sha256: "b".repeat(64),
            size: None,
        },
    );
    assert!(matches!(panel.message(), Some(Message::Failed(_))));
    assert_eq!(panel.items().len(), 1);
}

#[test]
fn a_write_that_removes_a_signature_is_asked_about_and_another_writer_is_noticed() {
    let dir = tempdir().unwrap();
    let path = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&path));
    let mut other = model();
    other.set_project(Some(&path));
    run(
        &mut other,
        Request::AttachFile(file(dir.path(), "one.txt", b"made up")),
    );
    assert!(panel.check_file(), "what another program wrote is seen");
    assert_eq!(panel.items().len(), 1);
    assert!(!panel.check_file());

    Project::open(&path, &OpenOptions::default())
        .unwrap()
        .sign_project(&Ed25519Signer::from_seed(&[7; 32]), &SaveOptions::default())
        .unwrap();
    assert!(panel.check_file());
    let two = file(dir.path(), "two.txt", b"made up too");
    panel.request(Request::AttachFile(two));
    assert!(!panel.busy());
    assert_eq!(panel.pending().unwrap().lines, [Ask::SignedProject]);
    panel.confirm();
    wait(&mut panel);
    assert_eq!(panel.items().len(), 2);

    // another project drops what was waiting
    panel.request(Request::Remove(id_of(&panel, "two.txt")));
    assert!(panel.pending().is_some());
    panel.set_project(None);
    assert!(panel.pending().is_none() && panel.items().is_empty() && panel.message().is_none());
}

#[test]
fn a_file_that_is_not_a_project_and_a_project_not_written_yet() {
    let dir = tempdir().unwrap();
    let mut panel = model();
    panel.set_project(Some(&dir.path().join("later.eh3d")));
    assert!(panel.error().is_none() && panel.items().is_empty());
    let broken = file(dir.path(), "broken.eh3d", b"made up, not an archive");
    panel.set_project(Some(&broken));
    assert!(panel.error().is_some());
    assert!(!panel.apply_setting("other", "1"));
    assert!(panel.apply_setting("attachments.max_mb", "1"));
    let project = new_project(dir.path());
    panel.set_project(Some(&project));
    let big = file(dir.path(), "big.txt", &vec![b'a'; (1 << 20) + 1]);
    run(&mut panel, Request::AttachFile(big));
    assert!(matches!(panel.message(), Some(Message::Failed(_))));
    assert!(panel.items().is_empty());
    assert!(!Text::ALL.is_empty());
}

#[test]
fn dropped_files_go_in_one_after_another_and_ask_first_when_the_place_is_not_known() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let first = file(dir.path(), "first.txt", b"one");
    let second = file(dir.path(), "second.txt", b"two");
    let mut panel = model();
    // with no project nothing is kept
    panel.drop_files(std::slice::from_ref(&first), true);
    assert!(!panel.busy() && !panel.waiting());
    panel.set_project(Some(&project));
    // a folder is left out
    panel.drop_files(
        &[dir.path().to_path_buf(), first.clone(), second.clone()],
        true,
    );
    assert!(panel.busy() && panel.waiting());
    let until = Instant::now() + Duration::from_secs(60);
    while panel.busy() || panel.waiting() {
        assert!(Instant::now() < until, "the work did not end");
        std::thread::sleep(Duration::from_millis(5));
        panel.poll();
    }
    assert_eq!(panel.items().len(), 2);

    let third = file(dir.path(), "third.txt", b"three");
    panel.drop_files(std::slice::from_ref(&third), false);
    assert!(!panel.busy());
    let question = panel.pending().expect("a question");
    assert_eq!(question.lines, [Ask::Dropped("third.txt".to_owned())]);
    for lang in Lang::ALL {
        assert!(
            question.lines[0].text(lang).contains("third.txt"),
            "{lang:?}"
        );
    }
    panel.cancel();
    panel.poll();
    assert_eq!(panel.items().len(), 2);
    panel.drop_files(std::slice::from_ref(&third), false);
    panel.confirm();
    wait(&mut panel);
    assert_eq!(panel.items().len(), 3);

    // a refused file gives up the ones that wait, so what was refused stays said
    let secret = file(dir.path(), "own.cfg", b"server_token = made-up-value\n");
    let last = file(dir.path(), "last.txt", b"four");
    panel.drop_files(&[secret, last], true);
    wait(&mut panel);
    panel.poll();
    assert!(matches!(panel.message(), Some(Message::HoldsCredential(_))));
    assert!(!panel.waiting() && panel.items().len() == 3);
}

#[test]
fn the_hash_of_a_file_is_taken_apart_and_given_once() {
    let dir = tempdir().unwrap();
    let path = file(dir.path(), "abc.bin", b"abc");
    let mut panel = model();
    let wait_hash = |panel: &mut Model| {
        let until = Instant::now() + Duration::from_secs(60);
        while panel.hashing() {
            assert!(Instant::now() < until, "the hash did not end");
            std::thread::sleep(Duration::from_millis(5));
            panel.poll();
        }
    };
    panel.start_hash(path);
    wait_hash(&mut panel);
    let (hash, size) = panel.take_hash().expect("an answer").expect("a hash");
    assert_eq!(
        hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(size, 3);
    assert!(panel.take_hash().is_none() && panel.message().is_none());
    panel.start_hash(dir.path().join("not-there"));
    wait_hash(&mut panel);
    assert!(panel.take_hash().expect("an answer").is_err());
    assert!(matches!(panel.message(), Some(Message::Failed(_))));
    panel.start_hash(dir.path().join("not-there"));
    panel.drop_hash();
    assert!(!panel.hashing() && panel.take_hash().is_none());
}

#[test]
fn the_limit_of_the_settings_is_the_one_that_refuses_a_large_file() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let large = file(dir.path(), "large.bin", &vec![7u8; (1 << 20) + 1]);
    let mut panel = model();
    assert!(panel.apply_setting("attachments.max_mb", "1"));
    assert_eq!(panel.limits().max_attachment_bytes, 1 << 20);
    panel.set_project(Some(&project));
    run(&mut panel, Request::AttachFile(large.clone()));
    panel.confirm();
    wait(&mut panel);
    assert!(panel.items().is_empty(), "{:?}", panel.message());
    assert!(matches!(panel.message(), Some(Message::Failed(_))));
}

/// Another program saves the project: its write is made on a copy, which then takes its place.
fn another_program_attaches(project: &Path, source: &Path) {
    let apart = project.with_extension("apart.eh3d");
    fs::copy(project, &apart).unwrap();
    let mut other = model();
    other.set_project(Some(&apart));
    run(&mut other, Request::AttachFile(source.to_path_buf()));
    assert_eq!(other.message(), Some(&Message::Attached));
    fs::rename(&apart, project).unwrap();
}

#[test]
fn nothing_is_written_over_what_another_program_saved_since_the_panel_read() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&project));
    let theirs = file(dir.path(), "theirs.txt", b"theirs");
    let mine = file(dir.path(), "mine.txt", b"mine");
    another_program_attaches(&project, &theirs);
    let held = fs::read(&project).unwrap();

    run(&mut panel, Request::AttachFile(mine.clone()));
    assert_eq!(
        panel.message(),
        Some(&Message::ChangedOnDisk("made-up.eh3d".to_owned()))
    );
    assert!(panel.message().unwrap().is_warning());
    assert!(panel
        .message()
        .unwrap()
        .text(Lang::Es)
        .contains("made-up.eh3d"));
    assert_eq!(fs::read(&project).unwrap(), held);
    assert!(!panel.take_wrote());
    // the list was read again, and asking once more writes
    assert_eq!(panel.items().len(), 1);
    run(&mut panel, Request::AttachFile(mine));
    assert_eq!(panel.message(), Some(&Message::Attached));
    assert_eq!(panel.items().len(), 2);
}

#[test]
fn the_writes_of_the_same_program_one_after_another_are_not_another_program() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panels = [model(), model()];
    for panel in &mut panels {
        panel.set_project(Some(&project));
    }
    for (n, which) in [0, 1, 0].into_iter().enumerate() {
        let panel = &mut panels[which];
        let source = file(dir.path(), &format!("{n}.txt"), format!("{n}").as_bytes());
        run(panel, Request::AttachFile(source));
        assert_eq!(panel.message(), Some(&Message::Attached), "{n}");
    }
    assert_eq!(panels[0].items().len(), 3);
}

#[test]
fn a_question_stays_when_only_the_date_changed_and_goes_when_the_content_did() {
    let dir = tempdir().unwrap();
    let project = new_project(dir.path());
    let mut panel = model();
    panel.set_project(Some(&project));
    run(
        &mut panel,
        Request::AttachFile(file(dir.path(), "one.txt", b"one")),
    );
    let id = id_of(&panel, "one.txt");
    panel.request(Request::Remove(id.clone()));
    assert!(panel.pending().is_some());

    let touched = fs::File::options().write(true).open(&project).unwrap();
    touched
        .set_modified(std::time::SystemTime::now() + Duration::from_secs(3600))
        .unwrap();
    drop(touched);
    assert!(panel.check_file());
    assert!(panel.pending().is_some());

    another_program_attaches(&project, &file(dir.path(), "two.txt", b"two"));
    assert!(panel.check_file());
    assert!(panel.pending().is_none());
    assert!(matches!(panel.message(), Some(Message::ChangedOnDisk(_))));
    assert_eq!(panel.items().len(), 2);
}
