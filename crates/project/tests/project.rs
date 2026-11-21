//! Behaviour of the `.eh3d` project: branches, ownership, references, seals, signatures,
//! encryption, the optional records and the layout checks.

mod support;

use std::collections::BTreeSet;

use eh3d_container::{AtomicOptions, ContainerError, PkgCode};
use eh3d_project::spec::{BRANCH_FILE, FORMAT_VERSION};
use eh3d_project::{
    BranchUpdate, Checks, EncryptionRecord, OpenOptions, Project, SaveOptions, Seal, Writer,
};
use serde_json::json;
use support::*;
use tempfile::tempdir;

fn code_of<T>(result: Result<T, ContainerError>) -> PkgCode {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.code().expect("a refusal with a code"),
    }
}

fn open(path: &std::path::Path) -> Project {
    Project::open(path, &OpenOptions::default()).unwrap()
}

fn errors(findings: &[eh3d_project::Finding]) -> Vec<PkgCode> {
    findings
        .iter()
        .filter(|f| f.is_error())
        .map(|f| f.code)
        .collect()
}

#[test]
fn the_synthetic_project_has_every_branch_and_is_sound() {
    let dir = tempdir().unwrap();
    let mut p = synthetic(dir.path());
    let ids: Vec<&str> = p.branches().map(|b| b.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "3dp",
            "cad",
            "cnc",
            "common",
            "docs/pcb",
            "laser",
            "manufacturing",
            "pcb"
        ]
    );
    assert_eq!(p.record().format_version, FORMAT_VERSION);
    assert!(p.integrity_verified());
    let findings = p.diagnose(&Checks::default());
    assert_eq!(
        findings,
        vec![],
        "a freshly made project has nothing to report"
    );
    assert!(p.stale_refs().is_empty());
    // Reading the branch of another program is allowed.
    let outline = p.read_entry("pcb", "outline.json").unwrap();
    assert!(String::from_utf8(outline).unwrap().contains("rect 40x30"));
    assert_eq!(p.record().branches.len(), 8);
}

#[test]
fn saving_a_branch_keeps_the_bytes_of_every_other_entry() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let path = p.path().to_owned();
    let before = raw_entries(&path);
    let p = save(
        p,
        "laser",
        BranchUpdate::new("laser", "1.0", "0.2.0").with_entry("logo.json", b"{\"power\":99}\n"),
        "2000-01-02T00:00:00Z",
    );
    let after = raw_entries(&path);
    for (name, entry) in &before {
        if name.starts_with("laser/") || name == "project.json" || name == "integrity.json" {
            continue;
        }
        assert_eq!(after.get(name), Some(entry), "{name} changed");
    }
    assert_ne!(before["laser/logo.json"], after["laser/logo.json"]);
    assert_eq!(
        p.branch("laser")
            .unwrap()
            .record
            .as_ref()
            .unwrap()
            .app_version,
        "0.2.0"
    );
}

#[test]
fn opening_saving_the_same_and_opening_again_is_idempotent() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let path = p.path().to_owned();
    let first = std::fs::read(&path).unwrap();
    let hash = p.content_hash().unwrap();
    // The file is closed before it is replaced (Windows needs it).
    drop(p);
    let reopened = open(&path);
    assert_eq!(reopened.content_hash().unwrap(), hash);
    // The same branch again, with the same time: the very same file.
    let again = save(
        reopened,
        "pcb",
        pcb_update("rect 40x30"),
        "2000-01-01T00:00:08Z",
    );
    assert_eq!(std::fs::read(&path).unwrap(), first);
    assert_eq!(again.content_hash().unwrap(), hash);
}

#[test]
fn a_program_writes_only_its_own_branch() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let cad_writer = Writer::for_app("cad");
    let steal = pcb_update("hacked");
    let p = match p.save_branch(&cad_writer, steal, &SaveOptions::default()) {
        Ok(_) => panic!("cad wrote the branch of pcb"),
        Err(e) => {
            assert_eq!(e.code(), Some(PkgCode::BranchOwner));
            open(dir.path().join("synthetic.eh3d").as_path())
        }
    };
    // A program with the same claim but another name is refused too: the branch was written by pcb.
    let impostor = Writer::new("someone-else", &["pcb"]);
    assert_eq!(
        code_of(p.save_branch(&impostor, pcb_update("x"), &SaveOptions::default())),
        PkgCode::BranchOwner
    );
    // The file did not change in either case.
    let mut p = open(dir.path().join("synthetic.eh3d").as_path());
    assert!(
        String::from_utf8(p.read_entry("pcb", "outline.json").unwrap())
            .unwrap()
            .contains("rect 40x30")
    );
    // Anyone may write the shared branches.
    let any = Writer::new("another-program", &[]);
    let updated = p
        .save_branch(
            &any,
            meta_update().with_entry("notes.txt", b"hello"),
            &SaveOptions::default(),
        )
        .unwrap();
    assert!(updated
        .branch("common")
        .unwrap()
        .files
        .contains_key("notes.txt"));
}

#[test]
fn a_branch_that_is_not_known_is_kept_byte_for_byte() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("future.eh3d");
    let future_files: [(&str, &[u8]); 2] = [
        ("data.bin", &[0, 1, 2, 3, 255]),
        ("notes.md", b"a note from a newer program\n"),
    ];
    let (record, hash) = branch_json(
        "futureapp",
        &future_files,
        json!({"newer_field": {"a": [1, 2, 3]}}),
    );
    let pcb_files: [(&str, &[u8]); 1] = [("board.json", b"{}\n")];
    let (pcb_record, pcb_hash) = branch_json("pcb", &pcb_files, json!({}));
    let index = json!([
        {"id": "futureapp", "app": "futureapp", "format": "1.0", "app_version": "0.1.0", "content_hash": hash, "vendor_note": "kept"},
        {"id": "pcb", "app": "pcb", "format": "1.0", "app_version": "0.1.0", "content_hash": pcb_hash},
    ]);
    raw_project(
        &path,
        &project_json(index),
        &[
            ("futureapp/branch.json", record),
            ("futureapp/data.bin", vec![0, 1, 2, 3, 255]),
            (
                "futureapp/notes.md",
                b"a note from a newer program\n".to_vec(),
            ),
            ("pcb/branch.json", pcb_record),
            ("pcb/board.json", b"{}\n".to_vec()),
            ("x-vendor/blob", vec![9; 100]),
        ],
    );
    let before = raw_entries(&path);
    let mut p = open(&path);
    assert!(!p.branch("futureapp").unwrap().is_known());
    assert!(p.branch("x-vendor").unwrap().is_extension());
    assert_eq!(p.diagnose(&Checks::default()), vec![]);
    let p = p
        .save_branch(
            &Writer::for_app("pcb"),
            pcb_update("rect 1x1"),
            &SaveOptions::default(),
        )
        .unwrap();
    let after = raw_entries(&path);
    for name in [
        "futureapp/branch.json",
        "futureapp/data.bin",
        "futureapp/notes.md",
        "x-vendor/blob",
    ] {
        assert_eq!(before[name], after[name], "{name}");
    }
    // Even the fields of the index line that this crate does not know are still there.
    let line = p
        .record()
        .branches
        .iter()
        .find(|b| b.id == "futureapp")
        .unwrap();
    assert_eq!(line.extra["vendor_note"], json!("kept"));
    assert_eq!(
        p.branch("futureapp")
            .unwrap()
            .record
            .as_ref()
            .unwrap()
            .extra["newer_field"],
        json!({"a": [1, 2, 3]})
    );
}

#[test]
fn a_derived_branch_notices_that_its_source_changed_and_can_be_brought_up_to_date() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let p = save(p, "pcb", pcb_update("rect 50x30"), "2000-02-01T00:00:00Z");
    let mut p = p;
    let stale = p.stale_refs();
    assert_eq!(stale.len(), 1);
    assert_eq!(
        (
            stale[0].branch.as_str(),
            stale[0].dependency.branch.as_str()
        ),
        ("cad", "pcb")
    );
    // The 3dp print was derived from cad, which has not changed: not out of date.
    let findings = p.diagnose(&Checks::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, PkgCode::StaleDerived);
    assert!(!findings[0].is_error());
    assert_eq!(
        findings[0].diagnostic().severity,
        eh3d_diagnostic::Severity::Warning
    );
    // The program of cad derives again and saves with the reference to the new state.
    let dep = p
        .dependency_on("pcb", "outline2d", 1, vec!["case.json".into()])
        .unwrap();
    let mut case = BranchUpdate::new("cad", "1.0", "0.1.1")
        .with_entry("case.json", &canon(&json!({"inner": "rect 50x30"})));
    case.depends.push(dep);
    let mut p = save(p, "cad", case, "2000-02-02T00:00:00Z");
    assert!(p.stale_refs().iter().all(|s| s.branch != "cad"));
    // ... which in turn makes the print out of date.
    let findings = p.diagnose(&Checks::default());
    assert_eq!(
        findings
            .iter()
            .map(|f| f.branch.clone().unwrap())
            .collect::<Vec<_>>(),
        ["3dp"]
    );
}

#[test]
fn references_that_point_nowhere_are_refused_or_reported() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let mut bad = BranchUpdate::new("laser", "1.0", "0.1.0").with_entry("logo.json", b"{}\n");
    let mut dep = p.dependency_on("pcb", "outline2d", 1, vec![]).unwrap();
    dep.branch = "ghost".into();
    bad.depends.push(dep.clone());
    assert_eq!(
        code_of(p.save_branch(
            &Writer::for_app("laser"),
            bad.clone(),
            &SaveOptions::default()
        )),
        PkgCode::BranchRef
    );
    // A reference to itself, a malformed hash and a path outside the file are refused too.
    let p = open(dir.path().join("synthetic.eh3d").as_path());
    for (branch, hash) in [
        ("laser", "a".repeat(64)),
        ("pcb", "nothex".to_owned()),
        ("../pcb", "a".repeat(64)),
    ] {
        let mut b = BranchUpdate::new("laser", "1.0", "0.1.0");
        let mut d = dep.clone();
        d.branch = branch.into();
        d.hash = hash;
        b.depends.push(d);
        assert_eq!(
            code_of(
                Project::open(p.path(), &OpenOptions::default())
                    .unwrap()
                    .save_branch(&Writer::for_app("laser"), b, &SaveOptions::default())
            ),
            PkgCode::BranchRef
        );
    }
    // A file made by something else, whose record points to a branch that is not there.
    let path = dir.path().join("dangling.eh3d");
    let files: [(&str, &[u8]); 1] = [("a.json", b"{}\n")];
    let (record, hash) = branch_json(
        "cad",
        &files,
        json!({"depends": [{"branch": "pcb", "hash": "a".repeat(64), "format": "1.0", "artifact": "outline2d", "artifact_version": 1}]}),
    );
    let index = json!([{"id": "cad", "app": "cad", "format": "1.0", "app_version": "0.1.0", "content_hash": hash}]);
    raw_project(
        &path,
        &project_json(index),
        &[
            ("cad/branch.json", record),
            ("cad/a.json", b"{}\n".to_vec()),
        ],
    );
    let mut p = open(&path);
    assert_eq!(
        errors(&p.diagnose(&Checks::default())),
        [PkgCode::BranchRef]
    );
}

#[test]
fn faults_in_the_records_are_reported_with_their_codes() {
    let dir = tempdir().unwrap();
    let make_with = |name: &str,
                     record: Vec<u8>,
                     index_hash: &str,
                     index_format: &str,
                     extra: Vec<(&str, Vec<u8>)>| {
        let path = dir.path().join(name);
        let index = json!([{"id": "cad", "app": "cad", "format": index_format, "app_version": "0.1.0", "content_hash": index_hash}]);
        let mut entries = vec![
            ("cad/branch.json", record),
            ("cad/a.json", b"{}\n".to_vec()),
        ];
        entries.extend(extra);
        raw_project(&path, &project_json(index), &entries);
        path
    };
    let make = |name: &str, record: Vec<u8>, index_hash: &str, extra: Vec<(&str, Vec<u8>)>| {
        make_with(name, record, index_hash, "1.0", extra)
    };
    let files: [(&str, &[u8]); 1] = [("a.json", b"{}\n")];
    let (good, hash) = branch_json("cad", &files, json!({}));
    // A record that matches its files has nothing to report.
    let path = make("good.eh3d", good, &hash, vec![]);
    assert_eq!(open(&path).diagnose(&Checks::default()), vec![]);
    // The content hash of the record is another one.
    let other_files: [(&str, &[u8]); 1] = [("a.json", b"other")];
    let (lying, _) = branch_json("cad", &other_files, json!({}));
    let path = make("hash.eh3d", lying, &hash, vec![]);
    assert_eq!(
        errors(&open(&path).diagnose(&Checks::default())),
        [PkgCode::BranchHash]
    );
    // A file in the branch that the record did not hash.
    let (good, hash) = branch_json("cad", &files, json!({}));
    let path = make(
        "added.eh3d",
        good,
        &hash,
        vec![("cad/b.json", b"{}\n".to_vec())],
    );
    assert_eq!(
        errors(&open(&path).diagnose(&Checks::default())),
        [PkgCode::BranchHash, PkgCode::BranchHash]
    );
    // The record names another branch, has no valid format, or is not JSON at all.
    let (other, hash) = branch_json("pcb", &files, json!({}));
    let path = make("name.eh3d", other, &hash, vec![]);
    // the record names another branch, and it also disagrees with the index about the program
    assert_eq!(
        errors(&open(&path).diagnose(&Checks::default())),
        [PkgCode::BranchRecord, PkgCode::BranchRecord]
    );
    let (format, hash) = branch_json("cad", &files, json!({"format": "one"}));
    let path = make_with("format.eh3d", format, &hash, "one", vec![]);
    assert_eq!(
        errors(&open(&path).diagnose(&Checks::default())),
        [PkgCode::BranchRecord]
    );
    let path = make("json.eh3d", b"not json".to_vec(), &hash, vec![]);
    assert_eq!(
        errors(&open(&path).diagnose(&Checks::default())),
        [PkgCode::BranchRecord]
    );
    // A branch with no record at all.
    let path = dir.path().join("norecord.eh3d");
    raw_project(
        &path,
        &project_json(json!([])),
        &[("cad/a.json", b"{}\n".to_vec())],
    );
    let codes = errors(&open(&path).diagnose(&Checks::default()));
    assert!(codes.contains(&PkgCode::BranchRecord));
    // A newer record schema and a newer format of the branch.
    let (newer, hash) = branch_json("cad", &files, json!({"schema": 2, "format": "3.0"}));
    let path = make_with("newer.eh3d", newer, &hash, "3.0", vec![]);
    let mut p = open(&path);
    assert_eq!(
        errors(&p.diagnose(&Checks::default())),
        [PkgCode::BranchVersion]
    );
    let mut checks = Checks::default();
    checks.supported_formats.insert("cad".into(), (1, 4));
    let codes = errors(&p.diagnose(&checks));
    assert_eq!(codes, [PkgCode::BranchVersion, PkgCode::BranchVersion]);
}

#[test]
fn the_index_of_the_project_must_agree_with_the_branches() {
    let dir = tempdir().unwrap();
    let files: [(&str, &[u8]); 1] = [("a.json", b"{}\n")];
    let (record, hash) = branch_json("cad", &files, json!({}));
    let path = dir.path().join("index.eh3d");
    let index = json!([
        {"id": "cad", "app": "cad", "format": "1.0", "app_version": "0.1.0", "content_hash": "b".repeat(64)},
        {"id": "ghost", "app": "g", "format": "1.0", "app_version": "0.1.0", "content_hash": hash},
    ]);
    raw_project(
        &path,
        &project_json(index),
        &[
            ("cad/branch.json", record),
            ("cad/a.json", b"{}\n".to_vec()),
        ],
    );
    let codes = errors(&open(&path).diagnose(&Checks::default()));
    assert_eq!(codes, [PkgCode::BranchHash, PkgCode::BranchRecord]);
}

#[test]
fn a_newer_project_format_and_a_stray_file_are_refused_on_opening() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("newer.eh3d");
    let mut record = project_json(json!([]));
    record["format_version"] = json!("1.1.0");
    raw_project(&path, &record, &[]);
    assert_eq!(
        code_of(Project::open(&path, &OpenOptions::default())),
        PkgCode::NewerVersion
    );
    record["format_version"] = json!("2.0.0");
    raw_project(&dir.path().join("major.eh3d"), &record, &[]);
    assert_eq!(
        code_of(Project::open(
            &dir.path().join("major.eh3d"),
            &OpenOptions::default()
        )),
        PkgCode::NewerVersion
    );
    let stray = dir.path().join("stray.eh3d");
    raw_project(
        &stray,
        &project_json(json!([])),
        &[("stray.txt", b"x".to_vec())],
    );
    assert_eq!(
        code_of(Project::open(&stray, &OpenOptions::default())),
        PkgCode::OutsideRoots
    );
    let shallow = dir.path().join("shallow.eh3d");
    raw_project(
        &shallow,
        &project_json(json!([])),
        &[("docs/readme.md", b"x".to_vec())],
    );
    assert_eq!(
        code_of(Project::open(&shallow, &OpenOptions::default())),
        PkgCode::OutsideRoots
    );
    let upper = dir.path().join("upper.eh3d");
    raw_project(
        &upper,
        &project_json(json!([])),
        &[("Pcb/a", b"x".to_vec())],
    );
    assert_eq!(
        code_of(Project::open(&upper, &OpenOptions::default())),
        PkgCode::OutsideRoots
    );
    let mut container_rules = project_json(json!([]));
    container_rules["container_version"] = json!("2.0");
    raw_project(&dir.path().join("rules.eh3d"), &container_rules, &[]);
    assert_eq!(
        code_of(Project::open(
            &dir.path().join("rules.eh3d"),
            &OpenOptions::default()
        )),
        PkgCode::NewerVersion
    );
}

#[test]
fn a_changed_entry_fails_the_integrity_check_unless_it_is_switched_off() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let path = p.path().to_owned();
    drop(p);
    // Flip one bit of the stored data of an entry and keep everything else.
    flip_in_entry(&path, "pcb/outline.json");
    assert!(Project::open(&path, &OpenOptions::default()).is_err());
}

#[test]
fn a_sealed_branch_and_a_sealed_project_refuse_every_write() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let mut update = BranchUpdate::new("cnc", "1.0", "0.1.0").with_entry("part.json", b"{}\n");
    update.seal = Some(Seal {
        by: "cnc".into(),
        at: "2000-03-01T00:00:00Z".into(),
        reason: Some("delivered".into()),
    });
    let p = save(p, "cnc", update, "2000-03-01T00:00:00Z");
    assert!(p.branch("cnc").unwrap().is_sealed());
    let again = BranchUpdate::new("cnc", "1.0", "0.1.1").with_entry("part.json", b"{}\n");
    let path = p.path().to_owned();
    // (the project is consumed by the call: the file is closed whatever the outcome)
    assert_eq!(
        code_of(p.save_branch(&Writer::for_app("cnc"), again, &SaveOptions::default())),
        PkgCode::Sealed
    );
    // The other branches can still be saved.
    let p = save(
        open(&path),
        "laser",
        BranchUpdate::new("laser", "1.0", "0.1.1").with_entry("logo.json", b"{}\n"),
        "2000-03-02T00:00:00Z",
    );
    let p = p
        .seal_project(
            Seal {
                by: "owner".into(),
                at: "2000-03-03T00:00:00Z".into(),
                reason: None,
            },
            &SaveOptions::default(),
        )
        .unwrap();
    assert!(p.record().seal.is_some());
    assert_eq!(
        code_of(p.save_branch(
            &Writer::for_app("laser"),
            pcb_like("laser"),
            &SaveOptions::default()
        )),
        PkgCode::Sealed
    );
    let p = open(&path);
    assert_eq!(
        code_of(p.seal_project(
            Seal {
                by: "x".into(),
                at: "y".into(),
                reason: None
            },
            &SaveOptions::default()
        )),
        PkgCode::Sealed
    );
}

fn pcb_like(id: &str) -> BranchUpdate {
    BranchUpdate::new(id, "1.0", "9.9.9").with_entry("x.json", b"{}\n")
}

#[test]
fn signatures_are_made_with_a_key_that_is_never_in_the_file() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let signer = ToySigner("studio-key");
    let signed = SaveOptions {
        signer: Some(&signer),
        ..SaveOptions::default()
    };
    let mut p = p
        .save_branch(
            &Writer::for_app("laser"),
            BranchUpdate::new("laser", "1.0", "0.1.0").with_entry("logo.json", b"{}\n"),
            &signed,
        )
        .unwrap();
    let record = p.branch("laser").unwrap().record.clone().unwrap();
    let sig = record.signature.clone().unwrap();
    assert_eq!(
        (sig.algorithm.as_str(), sig.key_id.as_str()),
        ("toy-sha256", "studio-key")
    );
    // Verified with the key: valid; no finding. Required for laser: fine. Required for pcb: missing.
    let verifier = ToyVerifier(vec!["studio-key"]);
    let mut checks = Checks {
        verifier: Some(&verifier),
        ..Checks::default()
    };
    checks.require_signed.insert("laser".into());
    assert_eq!(p.diagnose(&checks), vec![]);
    checks.require_signed.insert("pcb".into());
    assert_eq!(errors(&p.diagnose(&checks)), [PkgCode::SignatureMissing]);
    // Another verifier that does not know the key: "required" cannot be met.
    let stranger = ToyVerifier(vec!["other-key"]);
    let mut checks = Checks {
        verifier: Some(&stranger),
        ..Checks::default()
    };
    checks.require_signed.insert("laser".into());
    assert_eq!(errors(&p.diagnose(&checks)), [PkgCode::SignatureMissing]);
    // Without a verifier a required signature cannot be judged either.
    let mut checks = Checks::default();
    checks.require_signed.insert("laser".into());
    assert_eq!(errors(&p.diagnose(&checks)), [PkgCode::SignatureMissing]);
    // A key with the same name but another secret makes the signature invalid.
    struct Wrong;
    impl eh3d_project::Verifier for Wrong {
        fn verify(&self, _: &str, _: &str, _: &[u8], _: &[u8]) -> eh3d_project::VerifyOutcome {
            eh3d_project::VerifyOutcome::Invalid
        }
    }
    let wrong = Wrong;
    let checks = Checks {
        verifier: Some(&wrong),
        ..Checks::default()
    };
    assert_eq!(errors(&p.diagnose(&checks)), [PkgCode::SignatureInvalid]);
    // Saving the branch again without signing drops the signature (it no longer covers it).
    let p = p
        .save_branch(
            &Writer::for_app("laser"),
            BranchUpdate::new("laser", "1.0", "0.1.1").with_entry("logo.json", b"{}\n"),
            &SaveOptions::default(),
        )
        .unwrap();
    assert!(p
        .branch("laser")
        .unwrap()
        .record
        .as_ref()
        .unwrap()
        .signature
        .is_none());
}

#[test]
fn the_project_record_can_be_signed_and_the_signature_breaks_with_any_change() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let signer = ToySigner("owner-key");
    let mut p = p.sign_project(&signer, &SaveOptions::default()).unwrap();
    let verifier = ToyVerifier(vec!["owner-key"]);
    let checks = Checks {
        verifier: Some(&verifier),
        require_project_signature: true,
        ..Checks::default()
    };
    assert_eq!(p.diagnose(&checks), vec![]);
    // Saving a branch changes the hashes the project record holds: the old signature is gone.
    let mut p = save(
        p,
        "laser",
        BranchUpdate::new("laser", "1.0", "0.1.2").with_entry("logo.json", b"{\"a\":1}\n"),
        "2000-04-01T00:00:00Z",
    );
    assert!(p.record().signature.is_none());
    assert_eq!(errors(&p.diagnose(&checks)), [PkgCode::SignatureMissing]);
}

#[test]
fn an_encrypted_branch_cannot_be_read_without_its_key_but_is_kept_and_extracted() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let mut secret = BranchUpdate::new("cnc", "1.0", "0.1.0").with_entry("part.bin", &[7, 7, 7, 7]);
    secret.encryption = Some(EncryptionRecord {
        algorithm: "test-cipher".into(),
        key_id: "customer-1".into(),
        parameters: json!({"nonce": "00"}),
    });
    let mut p = save(p, "cnc", secret, "2000-05-01T00:00:00Z");
    assert!(p.branch("cnc").unwrap().is_encrypted());
    assert_eq!(
        code_of(p.read_entry("cnc", "part.bin")),
        PkgCode::EncryptedBranch
    );
    assert!(p.read_entry("cnc", BRANCH_FILE).is_ok());
    // The other branches stay readable.
    assert!(p.read_entry("pcb", "board.json").is_ok());
    let out = dir.path().join("out");
    assert_eq!(p.extract_branch("cnc", &out).unwrap(), 2);
    assert_eq!(std::fs::read(out.join("part.bin")).unwrap(), [7, 7, 7, 7]);
    // Extracting twice never replaces a file.
    assert!(p.extract_branch("cnc", &out).is_err());
}

#[test]
fn the_optional_records_are_checked_when_they_are_there() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    // Metadata in a language the project does not have.
    let bad_meta = BranchUpdate::new("common", "1.0", "0.1.0").with_entry(
        "meta/meta.json",
        &canon(&json!({"schema": 1, "title": {"en": "x", "nl": "y"}})),
    );
    let mut p = save(p, "any", bad_meta, "2000-06-01T00:00:00Z");
    let findings = p.diagnose(&Checks::default());
    assert_eq!(errors(&findings), [PkgCode::BranchRecord]);
    assert_eq!(findings[0].entry.as_deref(), Some("common/meta/meta.json"));
    // A document index that lists a file that is not there.
    let p = save(p, "any", meta_update(), "2000-06-02T00:00:00Z");
    let bad_docs = BranchUpdate::new("docs/pcb", "1.0", "0.1.0")
        .with_entry("index.json", &canon(&json!({"schema": 1, "documents": [{"path": "missing.md", "kind": "markdown", "title": {"en": "t"}}]})));
    let mut p = save(p, "pcb", bad_docs, "2000-06-03T00:00:00Z");
    assert_eq!(
        errors(&p.diagnose(&Checks::default())),
        [PkgCode::BranchRecord]
    );
    // A conversion report whose original is not the file it describes.
    let mut update = pcb_update("rect 40x30");
    update
        .entries
        .insert("origin/board.example".into(), b"another file".to_vec());
    let mut p = save(p, "pcb", update, "2000-06-04T00:00:00Z");
    let findings = p.diagnose(&Checks::default());
    assert!(findings
        .iter()
        .any(|f| f.entry.as_deref() == Some("pcb/origin/report.json")
            && f.code == PkgCode::BranchRecord));
}

#[test]
fn saving_with_an_audit_time_keeps_the_content_hash_of_the_project() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let before = p.content_hash().unwrap();
    let same_pcb = pcb_update("rect 40x30");
    let opts = SaveOptions {
        modified: Some("2000-01-01T00:00:08Z".into()),
        audit_at: Some("2000-07-01T00:00:00Z".into()),
        ..SaveOptions::default()
    };
    let p = p
        .save_branch(&Writer::for_app("pcb"), same_pcb, &opts)
        .unwrap();
    assert_eq!(p.audit_entries(), ["audit/log.jsonl"]);
    // The audit entries are left out of the content hash: only the log differs.
    assert_eq!(p.content_hash().unwrap(), before);
    let path = p.path().to_owned();
    assert!(zip_text(&path, "audit/log.jsonl").contains("\"branch\":\"pcb\""));
    // A second save adds a second line.
    let mut p = p
        .save_branch(&Writer::for_app("pcb"), pcb_update("rect 40x30"), &opts)
        .unwrap();
    assert_eq!(zip_text(&path, "audit/log.jsonl").lines().count(), 2);
    assert_eq!(p.diagnose(&Checks::default()), vec![]);
}

#[test]
fn a_new_project_never_replaces_an_existing_file_and_needs_a_name() {
    let dir = tempdir().unwrap();
    let p = new_project(dir.path());
    let again = Project::create(
        p.path(),
        eh3d_project::NewProject {
            id: "x".into(),
            name: "n".into(),
            units: "mm".into(),
            created: None,
        },
        &OpenOptions::default(),
        &AtomicOptions::default(),
    );
    assert!(matches!(again, Err(ContainerError::Io(_))));
    let nameless = Project::create(
        &dir.path().join("b.eh3d"),
        eh3d_project::NewProject {
            id: "x".into(),
            name: " ".into(),
            units: "mm".into(),
            created: None,
        },
        &OpenOptions::default(),
        &AtomicOptions::default(),
    );
    assert_eq!(code_of(nameless), PkgCode::BranchRecord);
}

#[test]
fn malformed_updates_are_refused_before_anything_is_written() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let path = p.path().to_owned();
    let before = std::fs::read(&path).unwrap();
    let w = Writer::for_app("pcb");
    let try_it = |update: BranchUpdate| {
        let p = Project::open(&path, &OpenOptions::default()).unwrap();
        code_of(p.save_branch(&w, update, &SaveOptions::default()))
    };
    assert_eq!(
        try_it(BranchUpdate::new("pcb", "one", "0.1.0")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(BranchUpdate::new("pcb", "1.0", " ")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("branch.json", b"{}")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("../escape", b"{}")),
        PkgCode::InvalidName
    );
    assert_eq!(
        try_it(BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("con.txt", b"{}")),
        PkgCode::InvalidName
    );
    assert_eq!(
        try_it(BranchUpdate::new("Pcb", "1.0", "0.1.0")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(BranchUpdate::new("audit", "1.0", "0.1.0")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(BranchUpdate::new("docs", "1.0", "0.1.0")),
        PkgCode::BranchRecord
    );
    assert_eq!(
        try_it(
            BranchUpdate::new("pcb", "1.0", "0.1.0")
                .with_entry("A.json", b"{}")
                .with_entry("a.json", b"{}")
        ),
        PkgCode::DuplicateName
    );
    // Nothing of that touched the file, and no temporary file was left behind.
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let leftovers: BTreeSet<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(leftovers, BTreeSet::from(["synthetic.eh3d".to_owned()]));
}

#[test]
fn the_synthetic_sample_file_is_reproducible() {
    // `tests/data/synthetic.eh3d` is the committed sample (made up, no data of anyone). It is
    // rebuilt with `cargo test -p eh3d-project regenerate_sample -- --ignored`.
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/synthetic.eh3d");
    let Ok(committed) = std::fs::read(&sample) else {
        return;
    };
    let dir = tempdir().unwrap();
    let made = synthetic(dir.path());
    assert_eq!(std::fs::read(made.path()).unwrap(), committed);
    let mut opened = Project::open(&sample, &OpenOptions::default()).unwrap();
    assert_eq!(opened.diagnose(&Checks::default()), vec![]);
}

#[test]
#[ignore = "rewrites the committed sample"]
fn regenerate_sample() {
    let dir = tempdir().unwrap();
    let made = synthetic(dir.path());
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/synthetic.eh3d");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::copy(made.path(), target).unwrap();
}
