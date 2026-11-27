//! A write over a project that another program saved meanwhile is refused (`EH3D-PKG-025`), and
//! replacing a signature needs an agreement (`EH3D-PKG-026`). "Another program" is a copy of the
//! project that is saved apart and then put in its place, which this program did not write.

mod support;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use eh3d_container::{ContainerError, PkgCode};
use eh3d_project::appbranch::{self, BranchKit, Identity};
use eh3d_project::manufacturing::{
    keep_result, keep_result_since, layers_of, read_result, save_result_since, state, JobFile,
    SOURCE_BRANCH,
};
use eh3d_project::revision;
use eh3d_project::suiteview::{self, check_unchanged, is_changed_refusal, revision_of};
use eh3d_project::{
    BranchUpdate, Ed25519Signer, OpenOptions, Project, Revision, SaveOptions, Writer,
};
use support::*;
use tempfile::tempdir;

const KIT: BranchKit<'static> = BranchKit::new("laser", "1.0", "laser.json", "0.1.0");

fn open(path: &Path) -> Project {
    Project::open(path, &OpenOptions::default()).unwrap()
}

fn code_of<T>(result: Result<T, ContainerError>) -> PkgCode {
    match result {
        Err(ContainerError::Rejected(v)) => v.code,
        Err(other) => panic!("not a refusal: {other}"),
        Ok(_) => panic!("it was not refused"),
    }
}

fn save_cnc(project: Project, text: &[u8]) -> Result<Project, ContainerError> {
    let update = BranchUpdate::new("cnc", "1.0", "0.1.0").with_entry("part.json", text);
    project.save_branch(&Writer::for_app("cnc"), update, &SaveOptions::default())
}

/// Another program saves the project: its write is made on a copy, which then takes its place.
/// The old file is moved away first, which every system allows while it is open.
fn another_program_saves(path: &Path, text: &[u8]) {
    let apart = path.with_extension("apart.eh3d");
    fs::copy(path, &apart).unwrap();
    save_cnc(open(&apart), text).unwrap();
    let old = path.with_extension("old");
    fs::rename(path, &old).unwrap();
    fs::rename(&apart, path).unwrap();
    fs::remove_file(old).unwrap();
}

fn put_laser(path: &Path, text: &[u8]) {
    KIT.update(path, &Identity::default(), |_, copy| {
        copy.put("a.svg", text.to_vec())
    })
    .unwrap();
}

fn project(dir: &Path) -> PathBuf {
    synthetic(dir).path().to_path_buf()
}

#[test]
fn an_open_project_never_writes_over_what_another_program_saved() {
    let dir = tempdir().unwrap();
    let path = project(dir.path());
    let held = open(&path);
    let signing = open(&path);
    let sealing = open(&path);
    another_program_saves(&path, b"theirs");
    let theirs = fs::read(&path).unwrap();

    assert_eq!(code_of(save_cnc(held, b"mine")), PkgCode::ChangedOnDisk);
    let key = Ed25519Signer::from_seed(&[3; 32]);
    assert_eq!(
        code_of(signing.sign_project(&key, &SaveOptions::default())),
        PkgCode::ChangedOnDisk
    );
    let seal = eh3d_project::Seal {
        by: "someone".into(),
        at: "2000-01-01T00:00:00Z".into(),
        reason: None,
    };
    assert_eq!(
        code_of(sealing.seal_project(seal, &SaveOptions::default())),
        PkgCode::ChangedOnDisk
    );
    // nothing was written and no temporary file is left
    assert_eq!(fs::read(&path).unwrap(), theirs);
    assert!(fs::read_dir(dir.path()).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".eh3dtmp")));
    // opened again, it is written
    save_cnc(open(&path), b"mine").unwrap();
}

#[test]
fn a_file_taken_away_or_spoiled_is_not_made_again_by_an_old_handle() {
    let dir = tempdir().unwrap();
    let path = project(dir.path());
    let held = open(&path);
    let seen = held.revision();
    fs::remove_file(&path).unwrap();
    assert_eq!(code_of(save_cnc(held, b"mine")), PkgCode::ChangedOnDisk);
    assert!(!path.exists());
    assert_eq!(
        code_of(revision::check_unchanged(&path, &seen)),
        PkgCode::ChangedOnDisk
    );
    fs::write(&path, b"not a project").unwrap();
    assert_eq!(
        code_of(revision::check_unchanged(&path, &seen)),
        PkgCode::ChangedOnDisk
    );
}

#[test]
fn saving_twice_in_a_row_and_only_a_new_date_are_not_a_change() {
    let dir = tempdir().unwrap();
    let path = project(dir.path());
    let seen = open(&path).revision();
    assert_eq!(Revision::of(&path).unwrap(), seen);
    assert_eq!(Revision::parse(seen.as_str()), Some(seen.clone()));
    assert_eq!(Revision::parse("short"), None);

    // the same handle goes on, and what this program wrote is not another program
    let first = save_cnc(open(&path), b"one").unwrap();
    save_cnc(first, b"two").unwrap();
    revision::check_unchanged(&path, &seen).unwrap();
    appbranch::open_unchanged(&path, &seen).unwrap();

    // only the date: touched, or written again with the same bytes
    let now = Revision::of(&path).unwrap();
    let file = File::options().write(true).open(&path).unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(3600))
        .unwrap();
    drop(file);
    let held = open(&path);
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, bytes).unwrap();
    revision::check_unchanged(&path, &now).unwrap();
    save_cnc(held, b"three").unwrap();
}

#[test]
fn a_write_of_this_program_does_not_hide_one_of_another() {
    let dir = tempdir().unwrap();
    let path = project(dir.path());
    let seen = open(&path).revision();
    save_cnc(open(&path), b"mine").unwrap();
    another_program_saves(&path, b"theirs");
    // and this program writes on top of that, having read it
    save_cnc(open(&path), b"mine again").unwrap();
    assert_eq!(
        code_of(revision::check_unchanged(&path, &seen)),
        PkgCode::ChangedOnDisk
    );
}

#[test]
fn a_branch_kit_refuses_to_write_over_a_file_saved_since_it_was_read() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    put_laser(&path, b"<svg/>");
    let seen = open(&path).revision();
    // the program saves twice with the mark of the first reading: both are its own
    for text in [&b"<svg a/>"[..], b"<svg b/>"] {
        KIT.update_since(&path, &seen, |_, copy| copy.put("a.svg", text.to_vec()))
            .unwrap();
    }
    another_program_saves(&path, b"theirs");
    let theirs = fs::read(&path).unwrap();
    let why = KIT
        .update_since(&path, &seen, |_, _| panic!("nothing is asked of a refusal"))
        .err()
        .unwrap()
        .to_string();
    assert!(is_changed_refusal(&why), "{why}");
    assert_eq!(fs::read(&path).unwrap(), theirs);
}

#[test]
fn the_window_is_refused_when_another_program_saved_and_may_save_under_another_name() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    put_laser(&path, b"<svg/>");
    let mark = suiteview::view(&path, "laser").unwrap().own_hash.unwrap();
    assert_eq!(revision_of(&mark), Some(open(&path).revision()));
    let ask = |target: &Path| suiteview::warnings(target, Some(&path), Some(&mark), "laser");
    assert_eq!(ask(&path).unwrap(), []);

    // another branch only: the whole file counts
    another_program_saves(&path, b"theirs");
    let why = ask(&path).unwrap_err();
    assert!(is_changed_refusal(&why), "{why}");
    assert!(check_unchanged(&path, &mark).is_err());
    // another name is not refused, and carries what the other program saved
    let copy = dir.path().join("copy.eh3d");
    assert_eq!(ask(&copy).unwrap(), []);
    // read again, the window goes on
    let again = suiteview::view(&path, "laser").unwrap().own_hash.unwrap();
    check_unchanged(&path, &again).unwrap();
    assert_eq!(
        suiteview::warnings(&path, Some(&path), Some(&again), "laser").unwrap(),
        []
    );
    // taken away is a change too
    fs::remove_file(&path).unwrap();
    assert!(suiteview::warnings(&path, Some(&path), Some(&again), "laser").is_err());
    // a window of a project without its branch keeps a mark as well
    let other = synthetic_in(dir.path(), "other");
    let mark = suiteview::view(&other, "nothing")
        .unwrap()
        .own_hash
        .unwrap();
    assert!(mark.starts_with("-@"), "{mark}");
    assert_eq!(
        suiteview::warnings(&other, Some(&other), Some(&mark), "nothing").unwrap(),
        []
    );
}

/// A project with a job of two drill layers.
fn job_project(dir: &Path) -> (PathBuf, Vec<JobFile>) {
    let path = project(dir);
    let record = br#"{"layers":[{"path":"layers/a.drl","kind":"drill"},{"path":"layers/b.drl","kind":"drill"}]}"#;
    let update = BranchUpdate::new(SOURCE_BRANCH, "1.0", "0.1.0")
        .with_entry("project.json", &record[..])
        .with_entry("layers/a.drl", &b"M48 a"[..])
        .with_entry("layers/b.drl", &b"M48 b"[..]);
    open(&path)
        .save_branch(
            &Writer::for_app(SOURCE_BRANCH),
            update,
            &SaveOptions::default(),
        )
        .unwrap();
    let layers = layers_of(&path, "drill").unwrap();
    (path, layers)
}

fn mark_of(path: &Path) -> String {
    suiteview::view(path, "laser").unwrap().own_hash.unwrap()
}

#[test]
fn a_result_is_not_kept_over_what_another_program_saved_since_the_opening() {
    let dir = tempdir().unwrap();
    let (path, layers) = job_project(dir.path());
    let mark = mark_of(&path);
    let result = |text: &[u8]| vec![("a.txt".to_string(), text.to_vec())];
    // the mark of the opening lasts through the writes of this program, with and without it
    keep_result_since(&KIT, &path, &mark, &layers[0], "a.txt", b"one".to_vec()).unwrap();
    keep_result(&KIT, &path, &layers[1], "b.txt", b"two".to_vec()).unwrap();
    save_result_since(&KIT, &path, &mark, &layers, &result(b"three")).unwrap();
    keep_result_since(&KIT, &path, &mark, &layers[1], "b.txt", b"four".to_vec()).unwrap();
    assert_eq!(state(&KIT, &path).unwrap().results, ["a.txt", "b.txt"]);

    another_program_saves(&path, b"theirs");
    let theirs = fs::read(&path).unwrap();
    for why in [
        keep_result_since(&KIT, &path, &mark, &layers[0], "a.txt", b"mine".to_vec()),
        save_result_since(&KIT, &path, &mark, &layers, &result(b"mine")),
        // the refusal comes before anything else is judged
        save_result_since(&KIT, &path, &mark, &[], &[]),
    ] {
        let why = why.unwrap_err();
        assert!(is_changed_refusal(&why), "{why}");
    }
    assert_eq!(fs::read(&path).unwrap(), theirs);
    // read again, it is written
    let again = mark_of(&path);
    keep_result_since(&KIT, &path, &again, &layers[0], "a.txt", b"mine".to_vec()).unwrap();
    assert_eq!(read_result(&KIT, &path, "a.txt").unwrap(), b"mine");
    assert_eq!(read_result(&KIT, &path, "b.txt").unwrap(), b"four");
}

#[test]
fn a_text_that_is_no_mark_and_a_file_that_is_gone_keep_nothing() {
    let dir = tempdir().unwrap();
    let (path, layers) = job_project(dir.path());
    let before = fs::read(&path).unwrap();
    let mark = mark_of(&path);
    let revision = mark.split_once('@').unwrap().1.to_string();
    for bad in [
        String::new(),
        "-".into(),
        "@".into(),
        "-@".into(),
        revision.clone(),
        mark.to_uppercase(),
        mark[..mark.len() - 1].to_string(),
        format!("{mark}0"),
        format!("-@{}", "0".repeat(64)),
    ] {
        let why =
            keep_result_since(&KIT, &path, &bad, &layers[0], "a.txt", b"x".to_vec()).unwrap_err();
        assert!(is_changed_refusal(&why), "{bad:?}: {why}");
        assert_eq!(fs::read(&path).unwrap(), before, "{bad:?}");
    }
    // the mark of a project that had no branch of this program holds the revision all the same
    assert!(mark.starts_with("-@"), "{mark}");
    fs::remove_file(&path).unwrap();
    let why =
        keep_result_since(&KIT, &path, &mark, &layers[0], "a.txt", b"x".to_vec()).unwrap_err();
    assert!(is_changed_refusal(&why), "{why}");
    assert!(!path.exists());
}

fn synthetic_in(dir: &Path, name: &str) -> PathBuf {
    let folder = dir.join(name);
    fs::create_dir(&folder).unwrap();
    project(&folder)
}

#[test]
fn a_signature_is_replaced_only_when_agreed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    put_laser(&path, b"<svg/>");
    let (first, second) = (
        Ed25519Signer::from_seed(&[1; 32]),
        Ed25519Signer::from_seed(&[2; 32]),
    );
    let options = SaveOptions::default();
    let signed = open(&path)
        .sign_branch("laser", &first, &options)
        .unwrap()
        .sign_project(&first, &options)
        .unwrap();
    let key_of = |p: &Project, id| p.signature_to_replace(id).unwrap().key_id.clone();
    assert_eq!(key_of(&signed, Some("laser")), key_of(&signed, None));
    let one = key_of(&signed, None);
    let bytes = fs::read(&path).unwrap();
    // the same signature again takes nothing away
    let signed = signed.sign_project(&first, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);

    assert_eq!(
        code_of(signed.sign_branch("laser", &second, &options)),
        PkgCode::SignatureExists
    );
    assert_eq!(
        code_of(open(&path).sign_project(&second, &options)),
        PkgCode::SignatureExists
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);

    let replaced = open(&path)
        .sign_branch_replacing("laser", &second, &options)
        .unwrap()
        .sign_project_replacing(&second, &options)
        .unwrap();
    assert_ne!(key_of(&replaced, None), one);
    assert_eq!(key_of(&replaced, Some("laser")), key_of(&replaced, None));
    assert!(replaced.signature_to_replace(Some("none")).is_none());
}
