//! What a program does with its own branch: save, open, import with a report, take from another
//! branch and notice that a source changed.

mod support;

use std::fs;
use std::path::Path;

use eh3d_project::appbranch::{
    conversion_report, counted, extract, file_name, list, new_project_id, read_bounded, BranchKit,
    BranchResult, Conversion, Identity, Notice, TAKEN_DIR,
};
use eh3d_project::meta::{ORIGIN_DIR, REPORT_FILE};
use eh3d_project::{
    BranchUpdate, Ed25519Signer, OpenOptions, Project, SaveOptions, Signer, Writer,
};
use support::*;
use tempfile::tempdir;

const KIT: BranchKit<'static> = BranchKit::new("laser", "1.0", "laser.json", "0.1.0");

fn identity() -> Identity {
    Identity {
        name: Some("Example".into()),
        id: Some("example-id".into()),
    }
}

fn put_drawing(path: &Path, who: &Identity, bytes: &[u8]) -> BranchResult<Vec<Notice>> {
    let (_, notices) = KIT.update(path, who, |_, copy| {
        copy.put("drawing/a.svg", bytes.to_vec())?;
        copy.choose("drawing", "drawing/", "drawing/a.svg");
        copy.add_to_list("programs", "programs/a.nc");
        copy.add_to_list("programs", "programs/a.nc");
        Ok(())
    })?;
    Ok(notices)
}

#[test]
fn the_branch_is_saved_opened_and_what_is_not_touched_stays() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    assert!(put_drawing(&path, &identity(), b"<svg/>")
        .unwrap()
        .is_empty());
    let (_, copy, warnings) = KIT.open(&path).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(copy.text("drawing").as_deref(), Some("drawing/a.svg"));
    assert_eq!(copy.file("drawing/a.svg"), Some(&b"<svg/>"[..]));
    assert_eq!(copy.list("programs"), ["programs/a.nc"]);

    KIT.update(&path, &Identity::default(), |_, copy| {
        copy.put("settings/laser.cfg", b"x=1\n".to_vec())
    })
    .unwrap();
    let (project, copy, _) = KIT.open(&path).unwrap();
    assert_eq!(
        copy.files().len(),
        3,
        "the record, the drawing, the settings"
    );
    assert_eq!(project.record().name, "Example");

    // a name and an identifier are only for a new project, and a new one ends in .eh3d
    assert!(put_drawing(&path, &identity(), b"x").is_err());
    let wrong = dir.path().join("work.zip");
    assert!(put_drawing(&wrong, &Identity::default(), b"x").is_err());
    assert!(!wrong.exists());
    // names that cannot live in a project
    assert!(KIT.check_name("../x").is_err());
    assert!(KIT.check_name("branch.json").is_err());
    // another format of the branch is not touched
    let newer = BranchKit::new("laser", "2.0", "laser.json", "0.2.0");
    let message = newer.open(&path).err().unwrap().to_string();
    assert!(message.contains("EH3D-PKG-024"), "{message}");
}

#[test]
fn an_import_keeps_the_original_with_its_report_and_replaces_the_one_before() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    let import = |name: &str, bytes: &[u8]| {
        let conversion = Conversion {
            read: vec![counted("shapes", 2, None)],
            not_read: vec![counted("pictures", 1, Some("not a vector"))],
            export_losses: vec![],
        };
        let report = conversion_report("tests", "0.1.0", "svg", name, bytes, conversion);
        KIT.import(
            &path,
            &Identity::default(),
            bytes.to_vec(),
            &report,
            |c, o| {
                c.choose("drawing", "drawing/", o);
                Ok(())
            },
        )
    };
    assert!(import("first.svg", b"<svg a/>").unwrap().is_empty());
    let first = format!("{ORIGIN_DIR}first.svg");
    let (_, copy, warnings) = KIT.open(&path).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(copy.text("drawing"), Some(first.clone()));
    assert_eq!(copy.report().unwrap().unwrap().source.name, "first.svg");

    assert_eq!(
        import("second.svg", b"<svg b/>").unwrap(),
        [Notice::ImportReplaced]
    );
    let (_, copy, warnings) = KIT.open(&path).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(copy.file(&first).is_none());
    assert!(copy.file(REPORT_FILE).is_some());
    // a refusal of the program leaves the project as it was
    let before = fs::read(&path).unwrap();
    let failed = KIT.update(&path, &Identity::default(), |_, _| KIT.check_name("../x"));
    assert!(failed.is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn what_is_taken_from_another_branch_is_noticed_when_its_source_changes() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let path = p.path().to_owned();
    drop(p);
    let (target, notices) = KIT
        .take(
            &path,
            "pcb",
            "outline.json",
            "outline2d",
            |bytes| {
                assert!(!bytes.is_empty());
                Ok(())
            },
            |copy, target| copy.set("drawing", target),
        )
        .unwrap();
    assert_eq!(target, format!("{TAKEN_DIR}pcb/outline.json"));
    assert!(notices.is_empty());
    let (_, copy, warnings) = KIT.open(&path).unwrap();
    assert_eq!(copy.depends().len(), 1);
    assert_eq!(copy.depends()[0].uses, std::slice::from_ref(&target));
    assert!(!warnings.iter().any(|w| w.contains("'laser'")));

    let p = Project::open(&path, &OpenOptions::default()).unwrap();
    save(p, "pcb", pcb_update("rect 50x30"), "2000-05-01T00:00:00Z");
    let (_, _, warnings) = KIT.open(&path).unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("OUT OF DATE: 'laser' was derived from 'pcb'")),
        "{warnings:?}"
    );
    let listing = list(&path).unwrap();
    let row = listing.rows.iter().find(|r| r.id == "laser").unwrap();
    assert!(row.notes.contains(&"OUT OF DATE"), "{:?}", row.notes);

    // removing the only file of a reference removes the reference
    KIT.update(&path, &Identity::default(), |_, copy| {
        copy.remove(&target);
        copy.forget_missing(&["drawing"]);
        Ok(())
    })
    .unwrap();
    let (_, copy, _) = KIT.open(&path).unwrap();
    assert!(copy.depends().is_empty());
    assert_eq!(copy.text("drawing"), None);

    // refusals write nothing
    let before = fs::read(&path).unwrap();
    for (source, rel) in [("laser", "logo.json"), ("pcb", "none.json")] {
        assert!(KIT
            .take(&path, source, rel, "file", |_| Ok(()), |_, _| {})
            .is_err());
    }
    assert_eq!(fs::read(&path).unwrap(), before);

    let out = dir.path().join("out");
    assert!(extract(&path, "cnc", &out).unwrap() >= 2);
    assert!(out.join("part.json").is_file());
    assert!(extract(&path, "cnc", &out).is_err());
}

#[test]
fn a_change_says_that_the_signatures_are_gone() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.eh3d");
    put_drawing(&path, &identity(), b"<svg/>").unwrap();
    let key = Ed25519Signer::from_seed(&[7; 32]);
    let p = Project::open(&path, &OpenOptions::default()).unwrap();
    let p = p
        .sign_branch("laser", &key, &SaveOptions::default())
        .unwrap();
    let p = p.sign_project(&key, &SaveOptions::default()).unwrap();
    let (_, copy, _) = KIT.open(p.path()).unwrap();
    let signed = SaveOptions {
        signer: Some(&key),
        ..SaveOptions::default()
    };
    let (saved, notices) = KIT.write(p, copy, &signed).unwrap();
    assert_eq!(notices, [Notice::ProjectSignatureRemoved]);
    let record = saved.branch("laser").unwrap().record.clone().unwrap();
    assert_eq!(record.signature.unwrap().key_id, key.key_id());
    assert_eq!(
        put_drawing(&path, &Identity::default(), b"<svg c/>").unwrap(),
        [Notice::BranchSignatureRemoved]
    );
    assert!(Notice::BranchSignatureRemoved
        .to_string()
        .contains("signature of the branch"));
}

#[test]
fn the_helpers_bound_what_they_read_and_make_identifiers() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("f.bin");
    fs::write(&file, [0u8; 10]).unwrap();
    assert_eq!(read_bounded(&file, 10).unwrap().len(), 10);
    let message = read_bounded(&file, 9).err().unwrap().to_string();
    assert!(message.contains("larger than 9 bytes"), "{message}");
    assert!(read_bounded(&dir.path().join("none"), 9).is_err());
    assert_eq!(file_name(&file).unwrap(), "f.bin");
    let (a, b) = (new_project_id().unwrap(), new_project_id().unwrap());
    assert_ne!(a, b);
    assert_eq!(a.len(), 36);
    assert_eq!(a.as_bytes()[14], b'4');

    // a branch of someone else keeps its bytes
    let p = new_project(dir.path());
    let p = p
        .save_branch(
            &Writer::for_app("cad"),
            BranchUpdate::new("cad", "1.0", "0.1.0").with_entry("case.json", b"{}\n"),
            &SaveOptions::default(),
        )
        .unwrap();
    let path = p.path().to_owned();
    drop(p);
    let before = raw_entries(&path);
    put_drawing(&path, &Identity::default(), b"<svg/>").unwrap();
    let after = raw_entries(&path);
    assert_eq!(after.get("cad/case.json"), before.get("cad/case.json"));
    assert_eq!(after.get("cad/branch.json"), before.get("cad/branch.json"));
}
